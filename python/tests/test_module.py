# Copyright 2026 Asim Ihsan
#
# This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
# If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
#
# SPDX-License-Identifier: MPL-2.0

"""Tests for Secure Tunnel Python bindings."""

from secure_tunnel import __version__, parse


def test_version():
    """Test that version is available."""
    assert isinstance(__version__, str)
    assert __version__ != ""


def test_parse_simple_addition():
    """Test parsing simple addition expressions."""
    assert parse("1+2+3") == "6"
    assert parse("10+20+30") == "60"
    assert parse("100+200+300") == "600"


def test_parse_whitespace():
    """Test parsing expressions with whitespace."""
    assert parse(" 1 + 2 + 3 ") == "6"
    assert parse("\t1\t+\t2\t+\t3\t") == "6"
    assert parse("\n1\n+\n2\n+\n3\n") == "6"
