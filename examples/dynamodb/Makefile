mk-docker:
	docker build -t olpa/ddb_convert .

push-docker:
	VERSION=$$(awk -F'"' '/^version =/ {print $$2}' Cargo.toml) && \
	docker tag olpa/ddb_convert olpa/ddb_convert:$$VERSION && \
	docker tag olpa/ddb_convert olpa/ddb_convert:latest && \
	docker push olpa/ddb_convert:$$VERSION && \
	docker push olpa/ddb_convert:latest
