.PHONY: all
all: rust clean

.PHONY: rust
rust:
	-docker-compose run --rm rust

.PHONY: clean
clean:
	docker-compose stop
	docker-compose rm -f

.PHONY: ci
ci:
	docker-compose run --rm rust cargo test --verbose
