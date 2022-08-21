# Certs
Note: The certificates here are only for demo.

## Self Signed Cert Generation
If you want to generate CA and server certificates by yourself, please make sure the certificates are in x509 v3 format(since webpki only support v3, and there's no way to convert v1 to v3). By default openssl generate server certificate in x509 v1.

```bash
openssl genrsa -out rootCA.key 4096
openssl req -x509 -new -nodes -sha512 -days 3650 \
-subj "/C=CN/ST=Shanghai/L=Shanghai/O=Monoio/OU=TLSDemo/CN=monoio-ca" \
-key rootCA.key \
-out rootCA.crt

openssl genrsa -out server.key 4096
openssl req -sha512 -new \
-subj "/C=CN/ST=Shanghai/L=Shanghai/O=Monoio/OU=TLSDemoServer/CN=monoio.rs" \
-key server.key \
-out server.csr

cat > v3.ext <<-EOF
authorityKeyIdentifier=keyid,issuer
basicConstraints=CA:FALSE
keyUsage=digitalSignature, nonRepudiation, keyEncipherment, dataEncipherment
extendedKeyUsage=serverAuth
subjectAltName=@alt_names

[alt_names]
DNS.1=monoio.rs
EOF

openssl x509 -req -sha512 -days 3650 \
-extfile v3.ext \
-CA rootCA.crt -CAkey rootCA.key -CAcreateserial \
-in server.csr \
-out server.crt
```