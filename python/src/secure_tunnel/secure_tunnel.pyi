"""Type hints for Secure Tunnel Python bindings."""

__version__: str

def protocol_id_v1() -> str:
    """Return the stable v1 protocol identifier."""
    ...

def quic_alpn_v1() -> str:
    """Return the v1 QUIC ALPN value."""
    ...

def wss_subprotocol_v1() -> str:
    """Return the v1 WSS subprotocol value."""
    ...

def example_service_descriptor_json() -> str:
    """Return a sample service descriptor as JSON."""
    ...

def validate_service_descriptor_json(descriptor_json: str) -> None:
    """Validate a service descriptor JSON document."""
    ...

def normalize_service_descriptor_json(descriptor_json: str) -> str:
    """Validate and re-encode a service descriptor JSON document."""
    ...
