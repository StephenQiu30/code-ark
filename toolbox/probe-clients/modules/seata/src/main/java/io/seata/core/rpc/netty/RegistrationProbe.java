package io.seata.core.rpc.netty;

/** Seata 1.8.0 exposes channel acquisition within this package. It completes RegisterTM before returning. */
public final class RegistrationProbe {
    public static boolean registered(TmNettyRemotingClient client, String endpoint) {
        var channel = client.getClientChannelManager().acquireChannel(endpoint);
        return channel != null && channel.isActive();
    }
}
