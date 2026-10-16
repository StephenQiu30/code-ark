require 'digest'

def register(params)
end

def safe_text(value, limit)
  text = value.to_s
  return '[REDACTED_SENSITIVE_MESSAGE]' if text.match?(/(authorization|api[_-]?key|password|passwd|secret|credential|prompt|剧本)\s*[=:]/i)
  text.gsub(/Bearer\s+[^\s,;]+/i, 'Bearer [REDACTED]')
      .gsub(%r{https?://[^\s<>"']+}i, '[REDACTED_URL]')[0, limit]
end

def filter(event)
  source = event.get('lucky')
  original = event.to_hash
  valid = source.is_a?(Hash) && %w[service event log deployment error].all? { |key| source[key].nil? || source[key].is_a?(Hash) } &&
          source['service'].is_a?(Hash) && source['event'].is_a?(Hash) && source['schemaVersion'] == '1' &&
          source.dig('service', 'name').to_s.match?(/^lucky-[A-Za-z0-9._-]+$/) &&
          source.dig('event', 'id').to_s.match?(/^[a-f0-9]{64}$/) && source['message'].is_a?(String)
  timestamp = event.get('@timestamp')
  original.keys.each { |key| event.remove(key) unless key == '@metadata' }
  event.set('@timestamp', timestamp)
  event.set('schemaVersion', '1')
  unless valid
    event.set('[event][id]', Digest::SHA256.hexdigest(original.to_s))
    event.set('[event][kind]', 'pipeline_error')
    event.set('[event][dataset]', 'lucky.application')
    event.set('[service][name]', 'lucky-logstash')
    event.set('[log][level]', 'WARN')
    event.set('[error][code]', 'invalid_kafka_record')
    event.set('message', 'Invalid Kafka log record; raw content discarded')
    return [event]
  end
  event.set('[event][id]', source.dig('event', 'id'))
  event.set('[event][kind]', source.dig('event', 'kind') == 'pipeline_error' ? 'pipeline_error' : 'event')
  event.set('[event][dataset]', 'lucky.application')
  event.set('[service][name]', source.dig('service', 'name'))
  event.set('message', safe_text(source['message'], 8192))
  level = source.dig('log', 'level').to_s.upcase
  level = { 'WARNING' => 'WARN', 'CRITICAL' => 'FATAL' }.fetch(level, level)
  event.set('[log][level]', %w[TRACE DEBUG INFO WARN ERROR FATAL].include?(level) ? level : 'INFO')
  event.set('[log][logger]', safe_text(source.dig('log', 'logger'), 256)) if source.dig('log', 'logger')
  environment = source.dig('deployment', 'environment')
  event.set('[deployment][environment]', environment) if environment.to_s.match?(/^[A-Za-z0-9._-]{1,64}$/)
  action = source.dig('event', 'action')
  event.set('[event][action]', action) if action.to_s.match?(/^[A-Za-z0-9._-]{1,64}$/)
  %w[requestId taskId runId operationId traceId spanId].each do |key|
    value = source[key]
    event.set(key, value) if value.is_a?(String) && value.match?(/^[A-Za-z0-9._-]{1,128}$/)
  end
  %w[durationMs httpStatus].each do |key|
    value = source[key]
    event.set(key, value) if value.is_a?(Numeric) && value >= 0 && value <= 86400000
  end
  dropped = source['droppedLogCount']
  event.set('droppedLogCount', dropped) if dropped.is_a?(Integer) && dropped > 0 && dropped <= 2147483647
  event.set('httpMethod', source['httpMethod']) if source['httpMethod'].to_s.match?(/^(GET|POST|PUT|PATCH|DELETE|HEAD|OPTIONS)$/)
  event.set('httpRoute', safe_text(source['httpRoute'], 256)) if source['httpRoute']
  event.set('eventTime', source['eventTime'].to_s[0, 64]) if source['eventTime']
  stack = source.dig('error', 'stack_trace')
  if stack.is_a?(String)
    locations = stack.lines.filter_map do |line|
      frame = line.match(/\A\s+at ([A-Za-z0-9_.$\/]{1,256})\(([A-Za-z0-9_$.-]{1,128}\.(?:java|py)):(\d{1,7})\)\s*\z/)
      "\tat #{frame[1]}(#{frame[2]}:#{frame[3]})" if frame && frame[3].to_i > 0
    end.first(48).join("\n")
    event.set('[error][stack_trace]', locations[0, 8192]) unless locations.empty?
  end
  error_code = source.dig('error', 'code')
  event.set('[error][code]', error_code) if error_code.to_s.match?(/^[a-z_]{1,64}$/)
  error_type = source.dig('error', 'type')
  event.set('[error][type]', error_type) if error_type.to_s.match?(/^[A-Za-z0-9_.$]{1,128}$/)
  [event]
end

test 'rejects exception text disguised as stack frames and preserves safe counters' do
  in_event { { 'lucky' => { 'schemaVersion' => '1', 'service' => { 'name' => 'lucky-agent' }, 'event' => { 'id' => 'a' * 64 }, 'log' => { 'level' => 'WARNING' }, 'message' => 'safe', 'droppedLogCount' => 42, 'error' => { 'stack_trace' => "\tat com.vendor.Error(secret-provider-payload)\n\tat com.vendor.Error(Foo.java:42) secret\n\tat python.run(worker.py:17)" } } } }
  expect('only reconstructed location and live counter survive') do |events|
    events[0].get('[error][stack_trace]') == "\tat python.run(worker.py:17)" &&
      events[0].get('droppedLogCount') == 42 && events[0].get('[log][level]') == 'WARN' &&
      !events[0].to_hash.to_s.include?('secret')
  end
end

test 'rejects nested scalar objects without exposing content' do
  in_event { { 'lucky' => { 'schemaVersion' => '1', 'service' => 'private payload', 'message' => 'secret' } } }
  expect('safe quarantine') { |events| events.size == 1 && events[0].get('[event][kind]') == 'pipeline_error' && !events[0].to_hash.to_s.include?('secret') }
end

test 'preserves correlation and redacts labeled messages' do
  in_event { { 'lucky' => { 'schemaVersion' => '1', 'service' => { 'name' => 'lucky-agent' }, 'event' => { 'id' => 'a' * 64 }, 'message' => 'prompt=private script', 'requestId' => 'req-1' } } }
  expect('safe correlated event') { |events| events[0].get('requestId') == 'req-1' && events[0].get('message') == '[REDACTED_SENSITIVE_MESSAGE]' }
end
