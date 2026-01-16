all: ddb_convert_screencast.gif

generated.cast: ./generate_screencast.py
	./generate_screencast.py

ddb_convert_screencast.gif: generated.cast
	agg --theme kanagawa-dragon $< $@
