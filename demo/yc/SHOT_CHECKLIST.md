# YC recording checklist

## Before the take

- [ ] Run `"/Users/hamza/Documents/Pioneer RI/reyn-studio/demo/yc/demo.sh" prepare`.
- [ ] Confirm the capsule is visible and the source hash begins `ca2b82f7`.
- [ ] Confirm **Run** is blocked by the missing compatible verified 3D model.
- [ ] Keep `demo/yc/assets/fixture-manifest.json` open for the fallback shot.
- [ ] Close notifications, chat, email, password managers, and unrelated windows.
- [ ] Use a clean 16:9 desktop at 1920×1080 or 2560×1440; set display scaling before
  positioning the app.
- [ ] Keep the pointer visible and move it deliberately. Do not zoom the entire desktop.
- [ ] Read the narration once with a stopwatch; target 2:40.

## macOS capture

1. Open **System Settings → Privacy & Security → Screen & System Audio Recording** and
   enable the recorder. Restart it if macOS asks.
2. Press `⌘⇧5`, choose **Record Selected Portion**, and frame only the app plus the
   small manifest window used at 2:08.
3. Under **Options**, select the narration microphone, choose a local save folder, and
   turn off the floating thumbnail if it distracts.
4. Screen-only is the default: no camera bubble, title animation, music, or edited
   result montage.
5. Start capture, wait two silent seconds, then begin the 00:00 line. Stop at 2:40.

QuickTime Player’s **File → New Screen Recording** opens the same capture controls.

## Review before compression

- [ ] Duration is 2:30–2:50.
- [ ] Text, SHA-256 prefix, triangle counts, units, and model-gate copy are readable.
- [ ] No personal path, notification, token, credential, or unrelated project appears.
- [ ] No neural field is shown or described as completed inference.
- [ ] The defective STL visibly reports 48 open edges, or the contingency line is used.
- [ ] The close says the qualified model and release gates remain unfinished.
- [ ] Narration is intelligible; if not, remove audio and use `captions.srt`.

## Compress and enforce limits

```bash
"/Users/hamza/Documents/Pioneer RI/reyn-studio/demo/yc/scripts/compress_demo.sh" \
  "/Users/hamza/Desktop/reyn-studio-yc.mov" \
  "/Users/hamza/Desktop/reyn-studio-yc.mp4"
```

The script rejects a take outside 150–170 seconds and fails if the compressed output
exceeds 100,000,000 bytes. Watch the final MP4 end-to-end after compression.

## Reset for another take

```bash
"/Users/hamza/Documents/Pioneer RI/reyn-studio/demo/yc/demo.sh" reset
```

This stops only the recorded demo process and deletes only `demo/yc/.state/`.
