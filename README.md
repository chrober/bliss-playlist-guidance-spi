# bliss-playlist-guidance-spi

`bliss-playlist-guidance-spi` defines the provider-neutral service-provider
interface used by native Bliss-first hosts to obtain candidate guidance.
Independent addons can contribute global candidate preferences (for example
play-count preference) or contextual edge guidance (for example a Last.fm
relationship for a particular transition).

The current wire contract is **SPI v2**, versioned JSONL over stdin/stdout. The
host owns discovery, bounded score batching, guidance aggregation, hard
eligibility, and its own selection objective. An addon only returns bounded
guidance signals and diagnostics. Provider failures degrade to neutral guidance.

SPI v2 distinguishes immutable, hash-bound evidence artifacts (for example
Last.fm relations resolved by Better Call Bliss) from trusted read-only
resources (for example Lyrion's `persist.db`). Preparation deliberately carries
no full candidate inventory; providers see only the bounded candidates being
scored, including `lms_urlmd5` when a provider needs Lyrion identity lookup.

The crate deliberately contains no LMS, Last.fm, Bliss database, or network
implementation. It is the stable boundary shared by the current
`bliss-playlist-optimizer` multi-step pathfinding host and a future
`bliss-mixer` DSTM-candidate-ranking host, plus provider repositories.

Read [SPI.md](SPI.md) for the complete lifecycle, JSONL messages, field
semantics, compatibility rules, failure behavior, and worked examples. The
normative JSONL schema is
[`schemas/guidance-addon-spi-v2.schema.json`](schemas/guidance-addon-spi-v2.schema.json).
Provider IDs identify guidance sources (for example `lastfm-guidance` and
`playcount-guidance`); they are not network clients or scoring algorithms.
