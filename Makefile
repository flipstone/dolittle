RUSTC ?= rustc
CC ?= gcc
BINDGEN ?= rust-bindgen
SRC ?= src
LIB ?= lib

run: all
	./bin/dolittle

all: $(LIB)/libhttp_parser.a
	$(RUSTC) -o bin/dolittle $(SRC)/crate.rc

check:
	$(RUSTC) -o bin/dolittle-test --test $(SRC)/crate.rc
		bin/dolittle-test $(test)

clean:
	rm -rf bin/*
	rm -rf lib/*

http_parser.rs:
	@echo "*****************************************************"
	@echo "* Bindgen is not perfect, so if you need to regenerate"
	@echo "* http_parser.rs, you may need to make some manual edits."
	@echo "* Check for struct char members with bit sizes and combine"
	@echo "* the correspondingr rust fields into a single char."
	@echo "*****************************************************"

	$(BINDGEN) $(SRC)/http_parser.h > $(SRC)/http_parser.rs

$(LIB)/libhttp_parser.a:
	$(CC) -c $(SRC)/http_parser.c -o $(LIB)/libhttp_parser.a


