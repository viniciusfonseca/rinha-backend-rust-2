target:
	cargo build --release

image:
	docker build . -t distanteagle16/rinhabackend-2

push-image:
	docker push distanteagle16/rinhabackend-2