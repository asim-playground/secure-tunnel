"""Secure Tunnel Python bindings.

A simple arithmetic expression parser that can evaluate expressions like "1+2+3".

Example:
    >>> from secure_tunnel import parse
    >>> parse("1+2+3")
    '6'
"""

from .secure_tunnel import __version__, parse

__all__ = ["__version__", "parse"]
