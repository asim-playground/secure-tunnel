import 'package:flutter_test/flutter_test.dart';
import 'package:secure_tunnel_flutter/secure_tunnel_flutter.dart';

void main() {
  test('hand-written facade can be imported and faked', () async {
    final client = _FakeClient();
    final connection = await client.connect(
      const ConnectOptions(descriptorJson: '{}', nowUnixSeconds: 1),
    );
    expect(connection.report().selectedCarrier, Carrier.quic);
    expect(
      (await connection.authenticateAccount(
        const AccountAuthRequest(
          accountId: 'flutter-smoke',
          credentialPayload: <int>[1, 2, 3],
          mode: AccountAuthMode.fresh,
        ),
      )).accountId,
      'flutter-smoke',
    );
  });
}

final class _FakeClient implements SecureTunnelClientApi {
  @override
  Future<SecureTunnelConnectionApi> connect(ConnectOptions options) async =>
      _FakeConnection();
}

final class _FakeConnection implements SecureTunnelConnectionApi {
  @override
  Future<AccountAuthReport> authenticateAccount(
    AccountAuthRequest request,
  ) async => AccountAuthReport(accountId: request.accountId);

  @override
  Future<CloseReport> close({required int code, required bool drain}) async =>
      const CloseReport(classification: CloseClassification.graceful);

  @override
  Future<List<int>> request(List<int> payload) async => payload;

  @override
  ConnectReport report() => const ConnectReport(selectedCarrier: Carrier.quic);

  @override
  SecureChannelArtifacts securityArtifacts() =>
      const SecureChannelArtifacts(serviceStaticPublicKey: null);
}
