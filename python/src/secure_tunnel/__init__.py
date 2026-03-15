# Copyright 2026 Asim Ihsan
#
# This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
# If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
#
# SPDX-License-Identifier: MPL-2.0

"""Secure Tunnel Python bindings.

A simple arithmetic expression parser that can evaluate expressions like "1+2+3".

Example:
    >>> from secure_tunnel import parse
    >>> parse("1+2+3")
    '6'
"""

from .secure_tunnel import __version__, parse

__all__ = ["__version__", "parse"]
