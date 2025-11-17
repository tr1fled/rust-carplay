## Rust Carplay - AirPlay code

Welcome to the AirPlay section of the code, in here you will see code that uses gstreamer to process things like audio and video, however you must rewrite the code to your needs,

## Video
In `stream_processors` you can find an example that dumps screen feed to a file, this dumps the raw h.264 AnnexB bytestream to a file, which can then be viewed in a player like VLC, that should give you a rough idea of how to build your own 'sink'. 