# text2video Image/Text Video Plugin

Renders articles as vertical scrolling short videos, with a drafts box and processing history. Development conventions are in [`AGENTS.en.md`](AGENTS.en.md) in the same directory.

## Tools

| Tool     | Entry                          | Description                            |
| -------- | ------------------------------ | -------------------------------------- |
| Generate | `frontend/views/Generator.vue` | Manual/AI copy → video, supports batch |
| Drafts   | `frontend/views/Drafts.vue`    | Save, edit, batch generate             |
| History  | `frontend/views/History.vue`   | Open output, show location, delete     |

## Dependencies and Resources

- Video encoding depends on the system `ffmpeg`/`ffprobe`; assets such as fonts are located in the local layer `src-tauri/local-resources/`
  (for the download method see [`docs/en/local.md`](../../docs/en/local.md)).
