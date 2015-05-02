.PHONY: all
all: rust clean

.PHONY: rust
rust:
	-docker-compose run --rm rust

.PHONY: clean
clean:
	docker-compose stop
	docker-compose rm -f
