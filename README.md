# YouTube Downloader

A little program I made for my mum so that she could download videos from YouTube more easily. Not especially robust :^).

Due to the limitations of my mum's 2015 dualcore Macbook, the settings for
ffmpeg are optimised mainly for speed, not quality. If anyone wants to actually
use this, then looking at the arguments passed to ffmpeg in `process()` might be
a good idea.

## Roadmap
- [X] Proper(ish) error handling
- [X] Coloured console output
- [X] Possibly using the `dialoguer` or a similar crate for a better interactive mode
- [ ] Multiple downloads in a single run (multithreading?)
- [ ] Better error handling
- [ ] A GUI (`egui`?)
- [ ] A non-interactive CLI interface