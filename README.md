# bliss-playlist-guidance-spi

`bliss-playlist-guidance-spi` defines the provider-neutral service-provider
interface used by the native playlist optimizer to obtain candidate guidance.
Independent addons can contribute global candidate preferences (for example
play-count preference) or contextual edge guidance (for example a Last.fm
relationship for a particular transition).

The first wire contract is versioned JSONL over stdin/stdout. The optimizer
owns discovery, batching, guidance aggregation, hard eligibility, and route
validity. An addon only returns bounded guidance signals and diagnostics.
Provider failures are expected to degrade to neutral guidance.

The crate deliberately contains no LMS, Last.fm, Bliss database, or network
implementation. It is the stable boundary shared by the optimizer and addon
repositories.

The normative JSONL schema is
[`schemas/guidance-addon-spi-v1.schema.json`](schemas/guidance-addon-spi-v1.schema.json).
Provider IDs identify guidance sources (for example `lastfm-guidance` and
`playcount-guidance`); they are not network clients or scoring algorithms.
