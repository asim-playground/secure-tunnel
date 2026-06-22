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
    `java-library`
    `maven-publish`
}

group = "io.github.asimihsan"
version = "0.1.0"

repositories {
    mavenCentral()
}

kotlin {
    jvmToolchain(21)
}

dependencies {
    api("net.java.dev.jna:jna:5.17.0")
}

java {
    withSourcesJar()
}

publishing {
    publications {
        create<MavenPublication>("secureTunnelKotlin") {
            from(components["java"])
            artifactId = "secure-tunnel-kotlin"
            pom {
                name.set("Secure Tunnel Kotlin SDK")
                description.set("JVM package for the Secure Tunnel UniFFI Kotlin SDK")
                url.set("https://github.com/asim-playground/secure-tunnel")
                licenses {
                    license {
                        name.set("Mozilla Public License 2.0")
                        url.set("https://www.mozilla.org/MPL/2.0/")
                    }
                }
            }
        }
    }
    repositories {
        maven {
            name = "secureTunnelLocal"
            url = uri(
                providers.gradleProperty("secureTunnelMavenRepo")
                    .orElse(layout.buildDirectory.dir("repository").map { it.asFile.absolutePath })
                    .get(),
            )
        }
    }
}
