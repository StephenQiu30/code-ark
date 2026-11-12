pub fn console_protocol(service: &str, component: &str, port: u16) -> Option<&'static str> {
    if matches!(
        (service, component, port),
        ("monitoring", "grafana", 3000)
            | ("monitoring", "prometheus", 9090)
            | ("rabbitmq", "rabbitmq", 15672)
            | ("minio", "minio", 9001)
            | ("kafka", "kafka-ui", 8080)
            | ("rocketmq", "console", 8080)
            | ("temporal", "temporal-ui", 8080)
            | ("xxjob", "xxl-job-admin", 8080)
            | ("nacos", "nacos", 8080)
            | ("sentinel", "sentinel", 8858)
            | ("seata", "seata-server", 7091)
            | ("elastic", "kibana", 5601)
            | ("rsshub", "rsshub", 1200)
            | ("searxng", "searxng", 8080)
    ) {
        Some("http")
    } else {
        None
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_actual_console_ports_offer_browser_actions() {
        assert_eq!(
            console_protocol("monitoring", "grafana", 3000),
            Some("http")
        );
        assert_eq!(
            console_protocol("rabbitmq", "rabbitmq", 15672),
            Some("http")
        );
        assert_eq!(console_protocol("rabbitmq", "rabbitmq", 5672), None);
        assert_eq!(console_protocol("temporal", "temporal", 7233), None);
        assert_eq!(console_protocol("nacos", "nacos", 8848), None);
        assert_eq!(console_protocol("nacos", "nacos", 8080), Some("http"));
    }
}
