import 'dart:typed_data';

import 'rust/api.dart' as bridge;

enum Carrier {
  quic,
  wss;

  static Carrier fromBridge(bridge.FlutterCarrier value) => switch (value) {
    bridge.FlutterCarrier.quic => Carrier.quic,
    bridge.FlutterCarrier.wss => Carrier.wss,
  };
}

enum AccountAuthMode {
  fresh,
  resume;

  bridge.FlutterAccountAuthMode toBridge() => switch (this) {
    AccountAuthMode.fresh => bridge.FlutterAccountAuthMode.fresh,
    AccountAuthMode.resume => bridge.FlutterAccountAuthMode.resume,
  };
}

enum CloseClassification {
  graceful,
  abrupt,
  truncated;

  static CloseClassification fromBridge(
    bridge.FlutterCloseClassification value,
  ) => switch (value) {
    bridge.FlutterCloseClassification.graceful => CloseClassification.graceful,
    bridge.FlutterCloseClassification.abrupt => CloseClassification.abrupt,
    bridge.FlutterCloseClassification.truncated =>
      CloseClassification.truncated,
  };
}

final class ClientConfig {
  const ClientConfig({
    required this.quicReprobeDelaySeconds,
    required this.connectTimeoutMs,
    required this.quicConnectTimeoutMs,
    required this.wssConnectTimeoutMs,
    required this.secureReadyTimeoutMs,
    required this.recordReadTimeoutMs,
    required this.recordWriteTimeoutMs,
    required this.outerRootCertificatesDer,
    required this.descriptorTrustAnchors,
    required this.pinnedServiceStaticPublicKeys,
  });

  factory ClientConfig.defaults() => const ClientConfig(
    quicReprobeDelaySeconds: 300,
    connectTimeoutMs: 10000,
    quicConnectTimeoutMs: 3000,
    wssConnectTimeoutMs: 5000,
    secureReadyTimeoutMs: 5000,
    recordReadTimeoutMs: 10000,
    recordWriteTimeoutMs: 10000,
    outerRootCertificatesDer: <List<int>>[],
    descriptorTrustAnchors: <DescriptorTrustAnchor>[],
    pinnedServiceStaticPublicKeys: <List<int>>[],
  );

  final int quicReprobeDelaySeconds;
  final int connectTimeoutMs;
  final int quicConnectTimeoutMs;
  final int wssConnectTimeoutMs;
  final int secureReadyTimeoutMs;
  final int recordReadTimeoutMs;
  final int recordWriteTimeoutMs;
  final List<List<int>> outerRootCertificatesDer;
  final List<DescriptorTrustAnchor> descriptorTrustAnchors;
  final List<List<int>> pinnedServiceStaticPublicKeys;

  bridge.FlutterClientConfig toBridge() => bridge.FlutterClientConfig(
    quicReprobeDelaySeconds: BigInt.from(quicReprobeDelaySeconds),
    connectTimeoutMs: BigInt.from(connectTimeoutMs),
    quicConnectTimeoutMs: BigInt.from(quicConnectTimeoutMs),
    wssConnectTimeoutMs: BigInt.from(wssConnectTimeoutMs),
    secureReadyTimeoutMs: BigInt.from(secureReadyTimeoutMs),
    recordReadTimeoutMs: BigInt.from(recordReadTimeoutMs),
    recordWriteTimeoutMs: BigInt.from(recordWriteTimeoutMs),
    outerRootCertificatesDer: _uint8Lists(outerRootCertificatesDer),
    descriptorTrustAnchors: descriptorTrustAnchors
        .map((anchor) => anchor.toBridge())
        .toList(),
    pinnedServiceStaticPublicKeys: _uint8Lists(pinnedServiceStaticPublicKeys),
  );
}

final class DescriptorTrustAnchor {
  const DescriptorTrustAnchor({
    required this.keyId,
    required this.algorithm,
    required this.publicKey,
  });

  final String keyId;
  final String algorithm;
  final String publicKey;

  bridge.FlutterDescriptorTrustAnchor toBridge() =>
      bridge.FlutterDescriptorTrustAnchor(
        keyId: keyId,
        algorithm: algorithm,
        publicKey: publicKey,
      );
}

final class ConnectOptions {
  const ConnectOptions({
    required this.descriptorJson,
    required this.nowUnixSeconds,
  });

  final String descriptorJson;
  final int nowUnixSeconds;

  bridge.FlutterConnectOptions toBridge() => bridge.FlutterConnectOptions(
    descriptorJson: descriptorJson,
    nowUnixSeconds: BigInt.from(nowUnixSeconds),
  );
}

final class ConnectReport {
  const ConnectReport({required this.selectedCarrier});

  factory ConnectReport.fromBridge(bridge.FlutterConnectReport value) =>
      ConnectReport(selectedCarrier: Carrier.fromBridge(value.selectedCarrier));

  final Carrier selectedCarrier;
}

final class SecureChannelArtifacts {
  const SecureChannelArtifacts({required this.serviceStaticPublicKey});

  factory SecureChannelArtifacts.fromBridge(
    bridge.FlutterSecureChannelArtifacts value,
  ) => SecureChannelArtifacts(
    serviceStaticPublicKey: value.serviceStaticPublicKey,
  );

  final List<int>? serviceStaticPublicKey;
}

final class AccountAuthRequest {
  const AccountAuthRequest({
    required this.accountId,
    required this.credentialPayload,
    required this.mode,
  });

  final String accountId;
  final List<int> credentialPayload;
  final AccountAuthMode mode;

  bridge.FlutterAccountAuthRequest toBridge() =>
      bridge.FlutterAccountAuthRequest(
        accountId: accountId,
        credentialPayload: Uint8List.fromList(credentialPayload),
        mode: mode.toBridge(),
      );
}

final class AccountAuthReport {
  const AccountAuthReport({required this.accountId});

  factory AccountAuthReport.fromBridge(bridge.FlutterAccountAuthReport value) =>
      AccountAuthReport(accountId: value.accountId);

  final String accountId;
}

final class CloseReport {
  const CloseReport({required this.classification});

  factory CloseReport.fromBridge(bridge.FlutterCloseReport value) =>
      CloseReport(
        classification: CloseClassification.fromBridge(value.classification),
      );

  final CloseClassification classification;
}

List<Uint8List> _uint8Lists(List<List<int>> values) =>
    values.map(Uint8List.fromList).toList();
