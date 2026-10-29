# Printer CA sources

`bambulab_modern.pem` is the public `resources/cert/printer.cer` bundle from
[Bambu Studio](https://github.com/bambulab/BambuStudio/blob/master/resources/cert/printer.cer),
retrieved 2026-10-29. SHA256:
`168852cde67cd9c7648de5f95b46f7b950d1627966d2da6a968fd9ef9d034910`.

The bundle contains BBL CA, BBL CA2 RSA/ECC and cross-signed CA2 certificates.
X2D's N6-V2 intermediate is supplied by the printer, and chains to CA2 RSA.
Verified against the connected X2D with certificate validation and its configured
serial as the TLS hostname, including a TLS 1.2 handshake matching firmware.
The firmware continues checking the server certificate and hostname.

Existing legacy, P2S and H2C certificate selection is unchanged.
