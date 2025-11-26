CONVTOOL := ../../../examples/dynamodb/target/release/ddb_convert
# CONVTOOL := ../ddb_convert_rust/target/release/ddb_convert_rust
# CONVTOOL := python ../ddb_convert_python/ddb_convert.py

# Source files
SOURCE_FILES := $(wildcard original-normal/*.json)

# Target files
ALL_TARGETS_TODDB := $(patsubst original-normal/%,normal-to-ddb/%,$(SOURCE_FILES))
ALL_TARGETS_FROMDDB := $(patsubst normal-to-ddb/%,ddb-to-normal/%,$(ALL_TARGETS_TODDB))
ALL_TARGETS_CHECKEQ := $(patsubst original-normal/%,check-eq/%,$(SOURCE_FILES))

all:
	@echo "Available targets:"
	@echo "  to-ddb    - Convert normal JSON to DynamoDB JSON"
	@echo "  from-ddb  - Convert DynamoDB JSON back to normal JSON"
	@echo "  check-eq  - Check if roundtrip conversion matches original"
	@echo "  clean     - Remove generated files and directories"

.PHONY: to-ddb
to-ddb: $(ALL_TARGETS_TODDB)

.PHONY: from-ddb
from-ddb: $(ALL_TARGETS_FROMDDB)

.PHONY: check-eq
check-eq: $(ALL_TARGETS_CHECKEQ)

normal-to-ddb/%.json: original-normal/%.json
	@echo "Converting $<"
	@mkdir -p $(dir $@)
	time $(CONVTOOL) to-ddb -i $< -o $@

ddb-to-normal/%.json: normal-to-ddb/%.json
	@echo "Converting $<"
	@mkdir -p $(dir $@)
	time $(CONVTOOL) from-ddb -i $< -o $@

check-eq/%.json: original-normal/%.json ddb-to-normal/%.json
	@echo "Checking $<"
	../json-eq.sh $< ddb-to-normal/$(notdir $<)

.PHONY: clean
clean:
	rm -f normal-to-ddb/*.json
	rm -f ddb-to-normal/*.json
	rmdir normal-to-ddb ddb-to-normal 2>/dev/null || true
