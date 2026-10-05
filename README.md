# bliss-playlist-guidance-spi

`bliss-playlist-guidance-spi` defines the provider-neutral service-provider
interface used by native Bliss-first hosts to obtain candidate guidance.
Independent addons can contribute global candidate preferences (for example
play-count preference) or contextual edge guidance (for example a Last.fm
relationship for a particular transition).

## Current integration status

The SPI v2 contract is used by the released Library Signals and Last.fm
artifact-mode providers. Library Signals reads a trusted, read-only
`persist.db`; Last.fm currently consumes a hash-bound artifact prepared by the
Lyrion host and LastMix. Direct Last.fm API-key acquisition is not part of the
released native contract path yet. `bliss-playlist-optimizer` and the native
`bliss-mixer` guidance endpoint are the current hosts.

The current wire contract is **SPI v2**, versioned JSONL over stdin/stdout. The
host owns discovery, bounded score batching, guidance aggregation, hard
eligibility, and its own selection objective. An addon only returns bounded
guidance signals and diagnostics. Provider failures degrade to neutral guidance.

Provider channels may declare which host policies can consume their raw
observations. This keeps the source-specific provider independent of selection
math: `bounded_influence` applies a small signed per-candidate adjustment,
whereas `target_share` calibrates supported candidates within an existing
Bliss-qualified pool. Neither policy can admit a candidate that Bliss rejected.

SPI v2 distinguishes immutable, hash-bound evidence artifacts (for example
Last.fm relations resolved by Better Call Bliss) from trusted read-only
resources (for example Lyrion's `persist.db`). Preparation deliberately carries
no full candidate inventory; it carries only job anchors and provider input.
Providers see only the bounded candidates being scored, including `lms_urlmd5`
when a provider needs Lyrion identity lookup. The anchors let a provider map
the host's track context to its own identities, such as artist MBIDs to Last.fm
artist source IDs, without changing the host's route IDs.

The crate deliberately contains no LMS, Last.fm, Bliss database, or network
implementation. It is the stable boundary shared by the current
`bliss-playlist-optimizer` multi-step pathfinding host, the `bliss-mixer`
native guidance-host endpoint, and provider repositories. The first
`bliss-mixer` Library Signals vertical slice exposes bounded scoring and
`selection_trace_v1`. When a native provider is enabled, Bliss Mixer Lab
already sends its bounded DSTM candidate pool to that endpoint; Lab retains its
own selection policy and log formatter.

Read [SPI.md](SPI.md) for the complete lifecycle, JSONL messages, field
semantics, compatibility rules, failure behavior, and worked examples. The
normative JSONL schema is
[`schemas/guidance-addon-spi-v2.schema.json`](schemas/guidance-addon-spi-v2.schema.json).
Provider IDs identify guidance sources (for example `lastfm-guidance` and
`library-signals-guidance`); they are not network clients or scoring algorithms.

## Related implementations

- Host: [`bliss-playlist-optimizer`](https://github.com/chrober/bliss-playlist-optimizer)
- Last.fm provider: [`bliss-guidance-lastfm`](https://github.com/chrober/bliss-guidance-lastfm)
- Local library-signals provider: [`lms-guidance-library-signals`](https://github.com/chrober/lms-guidance-library-signals)

Each provider owns only its data source. The host owns Bliss-first candidate
admission, policy weighting, and the final selection objective.
