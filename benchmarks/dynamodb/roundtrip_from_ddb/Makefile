CONVTOOL := /home/olpa/p/streaming_json/examples/dynamodb/target/debug/ddb_convert

# Source files
SOURCE_FILES := $(wildcard original-ddb/*.json)

# Target files
ALL_TARGETS_FROMDDB := $(patsubst original-ddb/%,ddb-to-normal/%,$(SOURCE_FILES))
ALL_TARGETS_TODDB := $(patsubst ddb-to-normal/%,normal-to-ddb/%,$(ALL_TARGETS_FROMDDB))
ALL_TARGETS_CHECKEQ := $(patsubst original-ddb/%,check-eq/%,$(SOURCE_FILES))

all:
	@echo "Available targets:"
	@echo "  from-ddb  - Convert DynamoDB JSON to normal JSON"
	@echo "  to-ddb    - Convert normal JSON back to DynamoDB JSON"
	@echo "  check-eq  - Check if roundtrip conversion matches original"
	@echo "  clean     - Remove generated files and directories"

.PHONY: from-ddb
from-ddb: $(ALL_TARGETS_FROMDDB)

.PHONY: to-ddb
to-ddb: $(ALL_TARGETS_TODDB)

.PHONY: check-eq
check-eq: $(ALL_TARGETS_CHECKEQ)

ddb-to-normal/%.json: original-ddb/%.json
	@echo "Converting $<"
	@mkdir -p $(dir $@)
	time $(CONVTOOL) from-ddb -i $< -o $@

normal-to-ddb/%.json: ddb-to-normal/%.json
	@echo "Converting $<"
	@mkdir -p $(dir $@)
	time $(CONVTOOL) to-ddb -i $< -o $@

check-eq/%.json: original-ddb/%.json normal-to-ddb/%.json
	@echo "Checking $<"
	../json-eq.sh $< normal-to-ddb/$(notdir $<)

.PHONY: clean
clean:
	rm -f ddb-to-normal/*.json
	rm -f normal-to-ddb/*.json
	rmdir ddb-to-normal normal-to-ddb 2>/dev/null || true
