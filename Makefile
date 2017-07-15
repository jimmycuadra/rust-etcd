.PHONY: all
all: rust clean

.PHONY: rust
rust: ssl
	-docker-compose run --rm rust

.PHONY: clean
clean: clean-ssl
	docker-compose stop
	docker-compose rm -f

.PHONY: ci
ci: ssl
	docker-compose run --rm rust cargo test --verbose

.PHONY: ssl
ssl: tests/ssl/ca.der tests/ssl/client.pem tests/ssl/client.p12 tests/ssl/server.pem

.PHONY: clean-ssl
clean-ssl:
	rm -f tests/ssl/*.der tests/ssl/*.pem tests/ssl/*.srl tests/ssl/*.p12

tests/ssl/ca.der: tests/ssl/ca.pem
	openssl x509 -in tests/ssl/ca.pem -out tests/ssl/ca.der -outform der

tests/ssl/ca.pem: tests/ssl/ca-key.pem
	openssl req -x509 -new -nodes -key tests/ssl/ca-key.pem -days 10000 -out tests/ssl/ca.pem -subj "/CN=rust-etcd-test-ca"

tests/ssl/ca-key.pem:
	openssl genrsa -out tests/ssl/ca-key.pem 2048

tests/ssl/client.p12:
	openssl pkcs12 -export -out tests/ssl/client.p12 -inkey tests/ssl/client-key.pem -in tests/ssl/client.pem -certfile tests/ssl/ca.pem -password pass:secret

tests/ssl/client.pem: tests/ssl/ca.pem tests/ssl/ca-key.pem tests/ssl/client-csr.pem
	openssl x509 -req -in tests/ssl/client-csr.pem -CA tests/ssl/ca.pem -CAkey tests/ssl/ca-key.pem -CAcreateserial -out tests/ssl/client.pem -days 365

tests/ssl/client-csr.pem: tests/ssl/client-key.pem
	openssl req -new -key tests/ssl/client-key.pem -out tests/ssl/client-csr.pem -subj "/CN=rust-etcd-test-client"

tests/ssl/client-key.pem:
	openssl genrsa -out tests/ssl/client-key.pem 2048

tests/ssl/server.pem: tests/ssl/ca.pem tests/ssl/ca-key.pem tests/ssl/server-csr.pem
	openssl x509 -req -in tests/ssl/server-csr.pem -CA tests/ssl/ca.pem -CAkey tests/ssl/ca-key.pem -CAcreateserial -out tests/ssl/server.pem -days 365 -extensions v3_req -extfile tests/ssl/openssl.cnf

tests/ssl/server-csr.pem: tests/ssl/server-key.pem
	openssl req -new -key tests/ssl/server-key.pem -out tests/ssl/server-csr.pem -subj "/CN=rust-etcd-test-server" -config tests/ssl/openssl.cnf

tests/ssl/server-key.pem:
	openssl genrsa -out tests/ssl/server-key.pem 2048
