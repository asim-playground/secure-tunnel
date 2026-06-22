import 'rust/api.dart' as bridge;
import 'rust/frb_generated.dart';

import 'model.dart';

abstract interface class SecureTunnelClientApi {
  Future<SecureTunnelConnectionApi> connect(ConnectOptions options);
}

abstract interface class SecureTunnelConnectionApi {
  ConnectReport report();

  SecureChannelArtifacts securityArtifacts();

  Future<AccountAuthReport> authenticateAccount(AccountAuthRequest request);

  Future<List<int>> request(List<int> payload);

  Future<CloseReport> close({required int code, required bool drain});
}

final class SecureTunnelClient implements SecureTunnelClientApi {
  SecureTunnelClient._(this._inner);

  final bridge.SecureTunnelFlutterClient _inner;

  static Future<SecureTunnelClient> create({ClientConfig? config}) async {
    await RustLib.init();
    return SecureTunnelClient._(
      bridge.SecureTunnelFlutterClient.newInstance(
        config: (config ?? ClientConfig.defaults()).toBridge(),
      ),
    );
  }

  @override
  Future<SecureTunnelConnection> connect(ConnectOptions options) async {
    final inner = await _inner.connect(options: options.toBridge());
    return SecureTunnelConnection._(inner);
  }
}

final class SecureTunnelConnection implements SecureTunnelConnectionApi {
  SecureTunnelConnection._(this._inner);

  final bridge.SecureTunnelFlutterConnection _inner;

  @override
  ConnectReport report() => ConnectReport.fromBridge(_inner.report());

  @override
  SecureChannelArtifacts securityArtifacts() =>
      SecureChannelArtifacts.fromBridge(_inner.securityArtifacts());

  @override
  Future<AccountAuthReport> authenticateAccount(
    AccountAuthRequest request,
  ) async => AccountAuthReport.fromBridge(
    await _inner.authenticateAccount(request: request.toBridge()),
  );

  @override
  Future<List<int>> request(List<int> payload) =>
      _inner.request(payload: payload);

  @override
  Future<CloseReport> close({required int code, required bool drain}) async =>
      CloseReport.fromBridge(await _inner.close(code: code, drain: drain));
}
