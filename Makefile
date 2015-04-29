.PHONY: rust
rust:
	docker-compose run --rm rust

.PHONY: etcd
etcd:
	docker-compose up --no-recreate -d etcd
