# Local TLS version bound

Upstream: yanshay/esp-mbedtls, commit 93a643c0daf440c40d67fdacfc44d2ce3d8c7f16.
Only the small Rust wrapper is vendored. The pinned upstream sys bindings and static libraries remain unchanged.

Added `asynch::Session::set_max_version`, valid only before the handshake. This uses the same generated config fields as upstream minimum-version initialization. AMS-Hub caps only Anycubic connections at TLS 1.2; certificate/authentication settings and other connections are unchanged.

Reason: the tested Anycubic firmware issues a self-signed certificate with clientAuth EKU and no server name. It supports TLS 1.2, while this MbedTLS TLS 1.3 verification path treats client authmode as required. Setting a minimum of TLS 1.2 did not prevent TLS 1.3 negotiation, producing -0x2700 even in the existing local-certificate mode. Custom CA mode continues to verify certificates and must reject unsuitable chains.
