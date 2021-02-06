.PHONY: all clean

all:
	docker run --rm -v ${PWD}:/build -w /build pixix4/ev3dev-rust cargo build --release --target armv5te-unknown-linux-gnueabi