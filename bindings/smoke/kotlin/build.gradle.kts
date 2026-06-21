/*
 * Copyright 2026 Asim Ihsan
 *
 * This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
 * If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * SPDX-License-Identifier: MPL-2.0
 */

plugins {
    kotlin("jvm") version "2.3.21"
    application
}

kotlin {
    jvmToolchain(21)
}

dependencies {
    implementation("net.java.dev.jna:jna:5.17.0")
}

kotlin.sourceSets.main {
    kotlin.srcDir("../../../target/generated-bindings/uniffi/kotlin")
}

val ffiLib = providers.gradleProperty("secureTunnelFfiLib")
val fixtureProperties = providers.gradleProperty("secureTunnelFixtureProperties")

application {
    mainClass.set("SmokeKt")
    applicationDefaultJvmArgs = listOf(
        "-Duniffi.component.secure_tunnel_sdk_ffi.libraryOverride=${ffiLib.get()}",
        "-DsecureTunnelFixtureProperties=${fixtureProperties.get()}",
    )
}
