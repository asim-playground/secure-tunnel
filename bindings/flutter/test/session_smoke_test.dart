import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:secure_tunnel_flutter/secure_tunnel_flutter.dart';

void main() {
  test('connects through the Rust SDK facade', () async {
    final path = Platform.environment['SECURE_TUNNEL_FLUTTER_FIXTURE_JSON'];
    if (path == null || path.isEmpty) {
      throw StateError('missing SECURE_TUNNEL_FLUTTER_FIXTURE_JSON');
    }
    final fixture =
        jsonDecode(await File(path).readAsString()) as Map<String, Object?>;
    final defaults = ClientConfig.defaults();
    final config = ClientConfig(
      quicReprobeDelaySeconds: defaults.quicReprobeDelaySeconds,
      connectTimeoutMs: defaults.connectTimeoutMs,
      quicConnectTimeoutMs: defaults.quicConnectTimeoutMs,
      wssConnectTimeoutMs: defaults.wssConnectTimeoutMs,
      secureReadyTimeoutMs: defaults.secureReadyTimeoutMs,
      recordReadTimeoutMs: defaults.recordReadTimeoutMs,
      recordWriteTimeoutMs: defaults.recordWriteTimeoutMs,
      outerRootCertificatesDer: _decodeMany(
        fixture['outer_root_certificates_der_b64']! as List<Object?>,
      ),
      descriptorTrustAnchors: const <DescriptorTrustAnchor>[],
      pinnedServiceStaticPublicKeys: _decodeMany(
        fixture['pinned_service_static_public_keys_b64']! as List<Object?>,
      ),
    );
    final client = await SecureTunnelClient.create(config: config);
    final connection = await client.connect(
      ConnectOptions(
        descriptorJson: fixture['descriptor_json']! as String,
        nowUnixSeconds: fixture['now_unix_seconds']! as int,
      ),
    );

    expect(connection.report().selectedCarrier, Carrier.quic);
    final serviceKey = connection.securityArtifacts().serviceStaticPublicKey;
    expect(
      config.pinnedServiceStaticPublicKeys.any(
        (key) => _bytesEqual(key, serviceKey),
      ),
      isTrue,
    );

    final auth = await connection.authenticateAccount(
      const AccountAuthRequest(
        accountId: 'flutter-smoke',
        credentialPayload: <int>[
          99,
          114,
          101,
          100,
          101,
          110,
          116,
          105,
          97,
          108,
        ],
        mode: AccountAuthMode.fresh,
      ),
    );
    expect(auth.accountId, 'flutter-smoke');

    final response = await connection.request(
      base64Decode(fixture['smoke_ping_b64']! as String),
    );
    expect(response, base64Decode(fixture['smoke_pong_b64']! as String));

    final close = await connection.close(code: 1000, drain: true);
    stdout.writeln(
      jsonEncode(<String, String>{
        'language': 'flutter',
        'carrier': 'quic',
        'close': close.classification.name,
      }),
    );
  });
}

List<List<int>> _decodeMany(List<Object?> values) =>
    values.cast<String>().map(base64Decode).toList();

bool _bytesEqual(List<int> left, List<int>? right) {
  if (right == null || left.length != right.length) {
    return false;
  }
  for (var index = 0; index < left.length; index += 1) {
    if (left[index] != right[index]) {
      return false;
    }
  }
  return true;
}
