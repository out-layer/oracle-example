'use client';

import { useState } from 'react';
import Link from 'next/link';

const sections = [
  { id: 'overview', name: 'Overview' },
  { id: 'quick-start', name: 'Quick Start' },
  { id: 'governance', name: 'Governance & Security' },
  { id: 'data-freshness', name: 'Data Freshness & Attestation' },
  { id: 'verifiable-prices', name: 'Verifiable Signed Prices' },
  { id: 'pyth-native', name: 'Pyth Interface' },
  { id: 'direct-outlayer', name: 'Direct OutLayer Integration' },
  { id: 'price-oracle', name: 'Price Oracle Contract' },
  { id: 'wrapper-example', name: 'Integration Example' },
  { id: 'custom-data', name: 'Custom Data Sources' },
  { id: 'code-examples', name: 'Code Examples' },
  { id: 'deposits', name: 'Deposit Requirements' },
];

export default function DocsPage() {
  const [activeSection, setActiveSection] = useState('overview');

  const scrollToSection = (id: string) => {
    setActiveSection(id);
    const element = document.getElementById(id);
    if (element) {
      element.scrollIntoView({ behavior: 'smooth' });
    }
  };

  return (
    <div className="min-h-screen py-8 px-4">
      <div className="max-w-7xl mx-auto">
        <div className="lg:grid lg:grid-cols-[250px_1fr] lg:gap-8">
          {/* Sidebar Navigation */}
          <nav className="hidden lg:block sticky top-24 h-fit">
            <h3 className="text-sm font-semibold text-dark-400 uppercase tracking-wider mb-4">
              Documentation
            </h3>
            <ul className="space-y-1">
              {sections.map((section) => (
                <li key={section.id}>
                  <button
                    onClick={() => scrollToSection(section.id)}
                    className={`w-full text-left px-3 py-2 rounded-lg text-sm transition-colors ${
                      activeSection === section.id
                        ? 'bg-primary/20 text-primary'
                        : 'text-dark-400 hover:text-white hover:bg-dark-800'
                    }`}
                  >
                    {section.name}
                  </button>
                </li>
              ))}
            </ul>
            <div className="mt-8 pt-8 border-t border-dark-800">
              <a
                href="https://github.com/out-layer/oracle-example"
                target="_blank"
                rel="noopener noreferrer"
                className="text-dark-400 hover:text-white text-sm flex items-center"
              >
                View on GitHub
                <svg className="w-4 h-4 ml-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                </svg>
              </a>
            </div>
          </nav>

          {/* Main Content */}
          <main className="prose prose-invert max-w-none">
            {/* Overview */}
            <section id="overview" className="mb-16">
              <h1 className="text-4xl font-bold text-white mb-4">
                TEE-Secured Price Oracle
              </h1>
              <p className="text-xl text-dark-300 mb-8">
                On-Demand Oracle with Sustainable Economics — Based on OutLayer
              </p>

              <div className="card mb-8">
                <h3 className="text-lg font-semibold text-white mb-4">Key Features</h3>
                <ul className="space-y-2 text-dark-300">
                  <li>✓ On-demand delivery — your contract gets the price in a callback, no dependency on a shared feed</li>
                  <li>✓ Zero trust — all data processed inside Intel TDX enclave</li>
                  <li>✓ DAO-governed — all configuration managed through council proposals</li>
                  <li>✓ TEE-only signing — only keys generated inside TEE can push prices</li>
                  <li>✓ 16 price sources with median aggregation</li>
                  <li>✓ Native Pyth-compatible API — migrate from pyth-oracle.near by changing one address</li>
                  <li>✓ Custom data fetching from any HTTP API</li>
                  <li>✓ Subsidized mode — free calls when contract has funds</li>
                </ul>
              </div>

              <div className="grid md:grid-cols-3 gap-4">
                <div className="card text-center">
                  <div className="text-3xl font-bold text-primary mb-2">21</div>
                  <div className="text-dark-400 text-sm">Supported Tokens</div>
                </div>
                <div className="card text-center">
                  <div className="text-3xl font-bold text-primary mb-2">10</div>
                  <div className="text-dark-400 text-sm">Price Sources</div>
                </div>
                <div className="card text-center">
                  <div className="text-3xl font-bold text-primary mb-2">&lt;1s</div>
                  <div className="text-dark-400 text-sm">Response Time</div>
                </div>
              </div>
            </section>

            {/* Quick Start */}
            <section id="quick-start" className="mb-16">
              <h2 className="text-2xl font-bold text-white mb-6">Quick Start</h2>

              <div className="card border-green-500/50 mb-6">
                <div className="flex items-center gap-2 mb-4">
                  <span className="px-2 py-1 text-xs bg-green-500/20 text-green-400 rounded">Recommended</span>
                  <h3 className="text-lg font-semibold text-white">Get Prices with Callback</h3>
                </div>
                <p className="text-dark-400 text-sm mb-4">
                  Use <code className="text-primary">oracle_call</code> to request prices. Your contract receives data via <code className="text-primary">oracle_on_call</code> callback.
                </p>
                <pre className="bg-dark-950 rounded-lg p-4 overflow-x-auto">
                  <code className="text-green-400">
{`near call price-oracle.near oracle_call '{
  "receiver_id": "your-contract.near",
  "asset_ids": ["wrap.near", "eth.bridge.near"],
  "msg": ""
}' --accountId your.near --deposit 0.02 --gas 200000000000000`}
                  </code>
                </pre>
              </div>

              <h3 className="text-lg font-semibold text-white mb-4">Direct Price Request (no callback)</h3>
              <p className="text-dark-400 text-sm mb-4">For scripts and testing — returns prices directly.</p>
              <pre className="bg-dark-900 rounded-lg p-4 overflow-x-auto mb-6">
                <code className="text-green-400">
{`near call price-oracle.near request_price_data '{
  "asset_ids": ["wrap.near"]
}' --accountId your.near --deposit 0.02 --gas 200000000000000`}
                </code>
              </pre>

              <div className="card border-blue-500/30 bg-blue-500/5 mb-6">
                <div className="flex items-start gap-3">
                  <div className="text-blue-400 text-xl">i</div>
                  <div>
                    <h3 className="text-lg font-semibold text-white mb-2">About View Methods (get_price_data)</h3>
                    <p className="text-dark-400 text-sm mb-3">
                      <code className="text-primary">get_price_data</code> is a free view method, and it is the
                      one path that can hand you a stale price without saying so. On-chain writes are a separate,
                      much slower cycle than the off-chain feed, they cost gas, and they pause on their own when
                      the pushing account runs low — so a view can legitimately return something minutes old, or
                      nothing at all. <strong className="text-white">Check the timestamp it returns against your
                      own bound and fail closed</strong>; never treat a view as evidence of freshness.
                    </p>
                    <p className="text-dark-400 text-sm">
                      For anything that moves money, use <code className="text-primary">request_price_data</code>{' '}
                      (or <code className="text-primary">oracle_call</code>): the price is fetched in the enclave
                      for that call and delivered to your contract in a callback, so freshness is a property of the
                      request rather than of whatever happens to be stored.
                    </p>
                  </div>
                </div>
              </div>

              <h3 className="text-lg font-semibold text-white mb-4">Response Format</h3>
              <pre className="bg-dark-900 rounded-lg p-4 overflow-x-auto">
                <code className="text-blue-400">
{`{
  "timestamp": "1706889600000000000",
  "recency_duration_sec": 120,
  "prices": [
    {
      "asset_id": "wrap.near",
      "price": { "multiplier": "500000000", "decimals": 8 }
    }
  ]
}
// Price conversion: 500000000 / 10^8 = $5.00`}
                </code>
              </pre>
            </section>

            {/* Governance & Security */}
            <section id="governance" className="mb-16">
              <h2 className="text-2xl font-bold text-white mb-6">Governance & Security</h2>

              <div className="card border-green-500/30 bg-green-500/5 mb-6">
                <div className="flex items-start gap-3">
                  <div className="text-green-400 text-xl">✓</div>
                  <div>
                    <h3 className="text-lg font-semibold text-white mb-2">All Changes Go Through DAO</h3>
                    <p className="text-dark-400 text-sm">
                      Every contract state mutation — adding assets, configuring exchanges, registering push signers, upgrading the contract — requires a DAO council proposal with &gt;50% approval. No single key can modify the oracle.
                    </p>
                  </div>
                </div>
              </div>

              <h3 className="text-lg font-semibold text-white mb-4">TEE-Only Price Pushing</h3>
              <p className="text-dark-300 mb-4">
                Prices are pushed to the contract by <strong className="text-white">implicit accounts derived from TEE-generated keys</strong>.
                The private key is created inside the TEE (Intel TDX) and never leaves it — no human, including the project owner, ever sees it.
              </p>
              <div className="card bg-dark-900 mb-6">
                <pre className="text-sm text-dark-300 overflow-x-auto">
{`How PROTECTED_ keys work:

1. Project owner creates a secret (e.g., PROTECTED_KEY_RHEA) in OutLayer dashboard
2. Private key generated INSIDE TEE — never exposed to anyone
3. DAO proposal registers the derived implicit account as trusted oracle
4. Only this account can call report_prices for assigned assets
5. WASI code inside TEE signs transactions with the key

Result: No human holds the signing key. Only verified TEE code can push prices.`}
                </pre>
              </div>

              <h3 className="text-lg font-semibold text-white mb-4">DAO Proposal Actions</h3>
              <div className="overflow-x-auto mb-6">
                <table className="w-full text-left text-sm">
                  <thead>
                    <tr className="border-b border-dark-700">
                      <th className="py-3 px-4 text-dark-300">Action</th>
                      <th className="py-3 px-4 text-dark-300">Description</th>
                    </tr>
                  </thead>
                  <tbody className="text-dark-400">
                    <tr className="border-b border-dark-800">
                      <td className="py-3 px-4"><code className="text-primary">AddAsset / RemoveAsset</code></td>
                      <td className="py-3 px-4">Manage tracked assets</td>
                    </tr>
                    <tr className="border-b border-dark-800">
                      <td className="py-3 px-4"><code className="text-primary">SetAssetExchangeConfig</code></td>
                      <td className="py-3 px-4">Configure exchange tickers, Pyth/Chainlink feeds per asset</td>
                    </tr>
                    <tr className="border-b border-dark-800">
                      <td className="py-3 px-4"><code className="text-primary">RegisterPushSigner</code></td>
                      <td className="py-3 px-4">Register TEE-derived account as trusted price pusher</td>
                    </tr>
                    <tr className="border-b border-dark-800">
                      <td className="py-3 px-4"><code className="text-primary">ConfigureOutlayer</code></td>
                      <td className="py-3 px-4">Set OutLayer integration parameters</td>
                    </tr>
                    <tr className="border-b border-dark-800">
                      <td className="py-3 px-4"><code className="text-primary">UpgradeContract</code></td>
                      <td className="py-3 px-4">Contract upgrade via DAO vote (after <code>upload_upgrade_code</code>)</td>
                    </tr>
                  </tbody>
                </table>
              </div>

              <h3 className="text-lg font-semibold text-white mb-4">Self-Service for Projects</h3>
              <p className="text-dark-300 mb-4">
                Third-party projects can operate their own push signers:
              </p>
              <ol className="list-decimal list-inside text-dark-300 space-y-2 mb-4">
                <li>Create a TEE secret (<code className="text-primary">PROTECTED_KEY_*</code>) in OutLayer dashboard</li>
                <li>DAO proposal to register the key for specific assets</li>
                <li>Fund the derived implicit account with NEAR</li>
                <li>Scheduler pushes prices autonomously from TEE</li>
              </ol>

              <h3 className="text-lg font-semibold text-white mb-4">Anyone Can Push Prices On-Chain</h3>
              <p className="text-dark-300 mb-4">
                The on-chain update is permissionless. Anyone can call the worker with{' '}
                <code className="text-primary">update_prices</code> and{' '}
                <code className="text-primary">update_contract: true</code> and have fresh prices written to
                the oracle contract — paying only for the WASI execution, and working even when our scheduler
                is down. The feed does not depend on a single operator staying online.
              </p>
              <div className="card mb-4">
                <p className="text-dark-400 text-sm mb-3">
                  <strong className="text-white">A caller cannot influence the price.</strong> The worker fetches
                  and aggregates the sources itself inside the enclave, and the resulting{' '}
                  <code className="text-primary">report_prices</code> transaction is signed by a TEE-generated key
                  whose private half never leaves the enclave. The contract accepts a report only from the{' '}
                  <code className="text-primary">push_signer_accounts</code> registered for that asset, so a
                  caller-supplied price is rejected by construction.
                </p>
                <p className="text-dark-400 text-sm">
                  <strong className="text-white">Two limits bound the cost.</strong> An asset reported to the
                  contract less than <strong>20 seconds</strong> ago is skipped, so repeated triggers cannot spam
                  transactions. And gas comes from the push signer&apos;s implicit account: an empty balance simply
                  means no on-chain push, while prices in public storage keep updating either way.
                </p>
              </div>
            </section>

            {/* Data Freshness & Attestation */}
            <section id="data-freshness" className="mb-16">
              <h2 className="text-2xl font-bold text-white mb-6">Data Freshness & Attestation</h2>

              <p className="text-dark-300 mb-6">
                Every result is attested by OutLayer&apos;s TEE: the signature proves <em>this WASM binary produced this output inside Intel TDX</em>.
                For the full trust model — what the signature does and does not prove, and how to verify it — see the{' '}
                <a href="https://app.outlayer.ai/docs/tee-attestation" target="_blank" rel="noopener noreferrer" className="text-primary hover:underline">platform attestation docs</a>.
                This section covers what is specific to the oracle.
              </p>

              <h3 className="text-lg font-semibold text-white mb-4">Two ways prices reach your contract</h3>
              <div className="grid md:grid-cols-2 gap-4 mb-8">
                <div className="card">
                  <h4 className="text-white font-semibold mb-2">Pull — yield/resume</h4>
                  <p className="text-dark-400 text-sm">
                    Your contract calls <code className="text-primary">oracle_call</code> / <code className="text-primary">request_price_data</code>.
                    If the on-chain cache is fresh it returns immediately; if stale, the contract yields, the TEE fetches and returns prices inline, and execution resumes with the result.
                  </p>
                </div>
                <div className="card">
                  <h4 className="text-white font-semibold mb-2">Push — scheduled</h4>
                  <p className="text-dark-400 text-sm">
                    An off-chain scheduler triggers the TEE worker, which fetches, aggregates, and signs <code className="text-primary">report_prices</code> with a{' '}
                    <code className="text-primary">PROTECTED_</code> key generated inside the TEE. Prices stay warm in contract state for free <code className="text-primary">get_price_data</code> reads.
                  </p>
                </div>
              </div>

              <h3 className="text-lg font-semibold text-white mb-4">Generation time vs. source age</h3>
              <p className="text-dark-300 mb-4">
                A price is timestamped when the runner <em>read</em> the source, not by the age of the source&apos;s own data.
                Only Pyth exposes an upstream publish time that the oracle enforces; every other endpoint returns a value with no timestamp, so its reading is only as fresh as the fetch.
              </p>
              <div className="card mb-8 overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-dark-700 text-left text-dark-300">
                      <th className="py-2 px-4">Source</th>
                      <th className="py-2 px-4">Upstream timestamp</th>
                      <th className="py-2 px-4">Staleness check</th>
                    </tr>
                  </thead>
                  <tbody className="text-dark-400">
                    <tr className="border-b border-dark-800">
                      <td className="py-3 px-4"><code className="text-primary">pyth</code></td>
                      <td className="py-3 px-4">Yes (<code>publish_time</code>)</td>
                      <td className="py-3 px-4">Rejected if older than 120s</td>
                    </tr>
                    <tr className="border-b border-dark-800">
                      <td className="py-3 px-4">all others (coingecko, binance, chainlink, huobi, kucoin, gate, cryptocom, …)</td>
                      <td className="py-3 px-4">No</td>
                      <td className="py-3 px-4">Stamped with fetch time</td>
                    </tr>
                  </tbody>
                </table>
              </div>

              <h3 className="text-lg font-semibold text-white mb-4">On-chain freshness bounds</h3>
              <p className="text-dark-300 mb-4">
                Reads from contract state are bounded by several on-chain checks, so a consumer never silently gets an ancient value:
              </p>
              <ul className="list-disc list-inside text-dark-300 space-y-2 mb-4">
                <li><code className="text-primary">recency_duration_sec</code> — reports older than this window are ignored; a stale asset returns <code>price: null</code>.</li>
                <li>Majority-of-oracles quorum — a price is returned only if enough recent reports agree (median of recent reports).</li>
                <li><code className="text-primary">pyth_stale_threshold</code> (default 60s) — enforced by the Pyth-compatible getters.</li>
              </ul>
              <p className="text-dark-400 text-sm">
                For a cross-chain view call, include the source block height in the WASI output so your contract can enforce its own deadline — OutLayer attests whatever the program returns.
              </p>
            </section>

            {/* Verifiable Signed Prices */}
            <section id="verifiable-prices" className="mb-16">
              <h2 className="text-2xl font-bold text-white mb-6">Verifiable Signed Prices</h2>

              <p className="text-dark-300 mb-6">
                Pull prices over HTTPS with an <strong className="text-white">Ed25519 signature</strong> you verify
                yourself. You check the signature instead of trusting the transport, the operator, or us. Verification
                works off-chain today and on-chain whenever you want it to — the signed bytes are the same.
              </p>

              <h3 className="text-lg font-semibold text-white mb-4">Why you would want this</h3>
              <div className="grid md:grid-cols-2 gap-4 mb-6">
                <div className="card">
                  <h4 className="text-white font-semibold mb-2">No TEE infrastructure of your own</h4>
                  <p className="text-dark-400 text-sm">
                    Getting trustworthy prices normally means running your own enclave: attestation, key management,
                    node operations, and the cost of keeping it alive. Here that work is already done and attested —
                    you consume a signed feed and verify 64 bytes.
                  </p>
                </div>
                <div className="card">
                  <h4 className="text-white font-semibold mb-2">You stay in control of the on-chain write</h4>
                  <p className="text-dark-400 text-sm">
                    We do not push anything into your contract. You decide when to submit, at what cadence, and under
                    which conditions — and you pay that gas yourself. No dependency on a relayer that could stall,
                    disappear, or price its service however it likes.
                  </p>
                </div>
                <div className="card">
                  <h4 className="text-white font-semibold mb-2">The relayer does not have to be trusted</h4>
                  <p className="text-dark-400 text-sm">
                    Because the payload is signed at the source, whoever carries it cannot alter it. That can be your
                    own server, a keeper bot, or anyone else — the signature, not the messenger, is what your contract
                    checks.
                  </p>
                </div>
                <div className="card">
                  <h4 className="text-white font-semibold mb-2">One input among several</h4>
                  <p className="text-dark-400 text-sm">
                    Already reading other oracles? Use <code className="text-primary">exclude_sources</code> to drop
                    the ones you consume directly, and this feed stays genuinely independent rather than echoing a
                    price you already have.
                  </p>
                </div>
              </div>

              <div className="card border-primary/30 bg-primary/5 mb-8">
                <p className="text-dark-300 text-sm mb-3">
                  <strong className="text-white">What the signature proves.</strong> The feed is signed inside the
                  enclave with a key whose name starts with{' '}
                  <code className="text-primary">PROTECTED_</code>. That prefix is not a convention — OutLayer
                  generates such secrets <strong className="text-white">inside the TEE</strong>, and their value is
                  never shown to anyone, including the project owner (
                  <a href="https://app.outlayer.ai/docs/secrets#creating-secrets" target="_blank"
                     rel="noopener noreferrer" className="text-primary hover:underline">how PROTECTED_ secrets are created</a>).
                  A valid signature therefore means the payload came out of the attested binary, not from an operator
                  holding a key on a laptop. That key is <strong className="text-white">fixed in the worker&apos;s
                  source</strong> and signs nothing else — in particular it never signs a NEAR transaction, so a feed
                  signature can never be replayed as one, and no request can ask for a different signer.
                </p>
                <p className="text-dark-400 text-sm">
                  It does <strong className="text-white">not</strong> mean the price is correct — that follows from
                  auditing the (open-source) worker and from the sources it aggregates. Signature = origin, not truth.
                </p>
              </div>

              <h3 className="text-lg font-semibold text-white mb-4">Step 1 — Get the public key (once)</h3>
              <p className="text-dark-300 mb-4">
                Ask the worker for the public half of the signing key and pin it in your code or contract.
                <code className="text-primary"> get_public_key</code> is the one call that names a key:{' '}
                <code className="text-primary">key_name</code> selects which{' '}
                <code className="text-primary">PROTECTED_</code> secret to read, and it returns public
                material only.
              </p>
              <pre className="bg-dark-900 rounded-lg p-4 overflow-x-auto mb-4">
                <code className="text-sm text-dark-300">{`curl -sX POST https://api.outlayer.ai/call/price-oracle.near/price-oracle \\
  -H "X-Payment-Key: $PAYMENT_KEY" -H "Content-Type: application/json" \\
  -d '{
    "input": { "command": "get_public_key", "key_name": "PROTECTED_RHEA_FEED_KEY" },
    "secrets_ref": { "profile": "oracle", "account_id": "price-oracle.near" }
  }'`}</code>
              </pre>
              <p className="text-dark-400 text-sm mb-4">
                You do not have to take our word for that key — the call that produced it is recorded and attested,
                and both records show the request and the response side by side:
              </p>
              <div className="card mb-6">
                <ul className="text-dark-400 text-sm space-y-3">
                  <li>
                    <strong className="text-white">On-chain transaction.</strong> The execution is on NEAR, so the
                    input and the returned public key are both public and immutable —{' '}
                    <a href="https://nearblocks.io/txns/2ikGDGq6bKKCPhA2GkUWdvShXPmNue9zssownBBUPoU4/execution"
                       target="_blank" rel="noopener noreferrer" className="text-primary hover:underline">
                      example transaction
                    </a>. Anyone can read what was asked and what came back, years later.
                  </li>
                  <li>
                    <strong className="text-white">TEE attestation.</strong> The same call has a TDX attestation —{' '}
                    <a href="https://app.outlayer.ai/attestation/194943?network=mainnet"
                       target="_blank" rel="noopener noreferrer" className="text-primary hover:underline">
                      example attestation
                    </a>. Press <strong className="text-white">🔍 Load &amp; Verify from Blockchain</strong> there to
                    pull the input and output straight from the chain and check them against the hashes committed in
                    the quote. Since the Task Hash covers <code className="text-primary">output_hash</code>, a matching
                    quote proves this exact public key came out of the attested binary inside the enclave.
                  </li>
                </ul>
              </div>

              <h3 className="text-lg font-semibold text-white mb-4">Step 2 — Request signed prices</h3>
              <p className="text-dark-300 mb-4">
                Note there is <strong className="text-white">no key parameter here</strong>. Unlike{' '}
                <code className="text-primary">get_public_key</code>, this request cannot select a signing
                key: the feed is always signed with the one key above, fixed in the worker&apos;s source.
                A request that still sends <code className="text-primary">key_name</code> is accepted and the
                field is ignored, so older clients keep working.
              </p>
              <pre className="bg-dark-900 rounded-lg p-4 overflow-x-auto mb-4">
                <code className="text-sm text-dark-300">{`curl -sX POST https://api.outlayer.ai/call/price-oracle.near/price-oracle \\
  -H "X-Payment-Key: $PAYMENT_KEY" -H "Content-Type: application/json" \\
  -d '{
    "input": {
      "command": "get_signed_prices",
      "tokens": ["wrap.near", "eth.bridge.near", "usdt.tether-token.near"],
      "max_age_secs": 120,
      "exclude_sources": ["pyth"]
    },
    "secrets_ref": { "profile": "oracle", "account_id": "price-oracle.near" }
  }'`}</code>
              </pre>

              <div className="card mb-6 overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-dark-700 text-left text-dark-300">
                      <th className="py-2 px-3">Field</th>
                      <th className="py-2 px-3">Default</th>
                      <th className="py-2 px-3">Meaning</th>
                    </tr>
                  </thead>
                  <tbody className="text-dark-400">
                    <tr className="border-b border-dark-800">
                      <td className="py-2 px-3"><code className="text-primary">tokens</code></td>
                      <td className="py-2 px-3">required</td>
                      <td className="py-2 px-3">Only the assets you ask for are fetched, signed and billed — request one or twenty, the rest of the feed is not your concern</td>
                    </tr>
                    <tr className="border-b border-dark-800">
                      <td className="py-2 px-3"><code className="text-primary">max_age_secs</code></td>
                      <td className="py-2 px-3">120</td>
                      <td className="py-2 px-3">Your freshness window. It filters <strong>sources</strong>: the price is aggregated over exactly the venues observed within it, and <code>publish_time</code> is the oldest of them — so it is never larger than what you asked for. If too few venues qualify, we fetch fresh rather than serve a thinner set; if an asset still cannot be priced inside the window, the whole request fails instead of returning a stale entry</td>
                    </tr>
                    <tr className="border-b border-dark-800">
                      <td className="py-2 px-3"><code className="text-primary">exclude_sources</code></td>
                      <td className="py-2 px-3">none</td>
                      <td className="py-2 px-3">Drop sources you already consume yourself, so our feed stays an independent input. Unknown names are rejected, never ignored</td>
                    </tr>
                    <tr className="border-b border-dark-800">
                      <td className="py-2 px-3"><code className="text-primary">sig_format</code></td>
                      <td className="py-2 px-3"><code>json</code></td>
                      <td className="py-2 px-3"><code>json</code> or <code>borsh</code></td>
                    </tr>
                    <tr className="border-b border-dark-800">
                      <td className="py-2 px-3"><code className="text-primary">expo</code></td>
                      <td className="py-2 px-3">-8</td>
                      <td className="py-2 px-3">Price is an integer scaled by 10<sup>expo</sup></td>
                    </tr>
                    <tr className="border-b border-dark-800">
                      <td className="py-2 px-3"><code className="text-primary">min_sources_num</code></td>
                      <td className="py-2 px-3">1</td>
                      <td className="py-2 px-3">Minimum venues that must be inside your window for an asset to be priced. The default of <code>1</code> is a floor, not a recommendation — a lending market should raise it, so a narrow window can never be answered by a single venue</td>
                    </tr>
                  </tbody>
                </table>
              </div>

              <details className="card mb-6">
                <summary className="cursor-pointer text-white font-semibold">Example response (real output)</summary>
                <pre className="bg-dark-950 rounded-lg p-4 overflow-x-auto mt-4">
                  <code className="text-xs text-dark-300">{`{
  "success": true,
  "payload": "{\\"eth.bridge.near\\":{\\"price\\":\\"196837500000\\",\\"expo\\":-8,\\"publish_time\\":1785149621},\\"usdt.tether-token.near\\":{\\"price\\":\\"99908500\\",\\"expo\\":-8,\\"publish_time\\":1785149622},\\"wrap.near\\":{\\"price\\":\\"184283333\\",\\"expo\\":-8,\\"publish_time\\":1785149621}}",
  "signature": "YC2Nd2IyViEp7JKIDqkehHT9bnn2qTJl8iP0frNrlK63NJTjNO0LbI8u28qmH66+mEP2IIi+NrC4oIkXxk/uBw==",
  "public_key": "ed25519:FU6EnB4UaAiDCAxvQPkRUu5QQExgzvKQAX891wMEX3rU",
  "sig_format": "json",
  "error": null
}`}</code>
                </pre>
                <p className="text-dark-400 text-sm mt-3">
                  <code className="text-primary">payload</code> is a <strong className="text-white">string</strong>,
                  not an object — it is the signed message. Keys are the oracle&apos;s own asset ids, sorted, so the
                  bytes are reproducible.
                  <code className="text-primary"> price</code> is an integer sent as a string (it is an i64; a JSON
                  number would lose precision in some parsers). Real price ={' '}
                  <code className="text-primary">price × 10^expo</code>, e.g. 184283333 × 10⁻⁸ = $1.84283333.
                  <code className="text-primary"> publish_time</code> is the unix second at which the enclave observed
                  the oldest venue that contributed to that asset&apos;s price — the age of the aggregate, not the time
                  of your request. See{' '}
                  <a href="#publish-time" className="text-primary hover:underline">what publish_time is</a>.
                </p>
              </details>

              <div className="card border-red-500/30 bg-red-500/5 mb-6">
                <p className="text-dark-300 text-sm">
                  <strong className="text-white">The one rule that breaks integrations:</strong> verify the signature
                  over the <strong className="text-white">exact bytes of the <code>payload</code> string</strong>.
                  Do not parse it and re-serialize before verifying — key order, whitespace and number formatting will
                  differ and the signature will fail. Parse it only <em>after</em> the signature checks out. For{' '}
                  <code className="text-primary">borsh</code>, verify over{' '}
                  <code className="text-primary">base64_decode(payload)</code>, not over the base64 text.
                </p>
              </div>

              <details className="card mb-6">
                <summary className="cursor-pointer text-white font-semibold">Verifying the signature (JavaScript / Python / Rust)</summary>
                <div className="mt-4">
                  <p className="text-dark-400 text-sm mb-2">JavaScript (Node 18+, no dependencies beyond a base58 helper):</p>
                  <pre className="bg-dark-950 rounded-lg p-4 overflow-x-auto mb-4">
                    <code className="text-xs text-dark-300">{`import { verify, createPublicKey } from 'node:crypto';
import bs58 from 'bs58';

const PINNED = 'ed25519:FU6EnB4UaAiDCAxvQPkRUu5QQExgzvKQAX891wMEX3rU';

function verifyFeed(res) {
  if (res.public_key !== PINNED) throw new Error('unexpected signing key');

  // DER-wrap the raw 32-byte key so node's crypto can import it
  const raw = bs58.decode(res.public_key.split(':')[1]);
  const der = Buffer.concat([Buffer.from('302a300506032b6570032100', 'hex'), raw]);
  const key = createPublicKey({ key: der, format: 'der', type: 'spki' });

  const message = Buffer.from(res.payload, 'utf8');       // EXACT bytes, no re-serialize
  if (!verify(null, message, key, Buffer.from(res.signature, 'base64')))
    throw new Error('bad signature');

  const prices = JSON.parse(res.payload);                  // safe only after verifying
  const now = Math.floor(Date.now() / 1000);
  for (const [asset, p] of Object.entries(prices)) {
    if (now - p.publish_time > 120) throw new Error(\`\${asset} too old\`);
  }
  return prices;
}`}</code>
                  </pre>

                  <p className="text-dark-400 text-sm mb-2">Python:</p>
                  <pre className="bg-dark-950 rounded-lg p-4 overflow-x-auto mb-4">
                    <code className="text-xs text-dark-300">{`import base64, base58, json
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey

PINNED = "ed25519:FU6EnB4UaAiDCAxvQPkRUu5QQExgzvKQAX891wMEX3rU"

def verify_feed(res):
    assert res["public_key"] == PINNED, "unexpected signing key"
    vk = Ed25519PublicKey.from_public_bytes(base58.b58decode(PINNED.split(":", 1)[1]))
    vk.verify(base64.b64decode(res["signature"]), res["payload"].encode())  # raises if invalid
    return json.loads(res["payload"])   # parse only after the signature is verified`}</code>
                  </pre>

                  <p className="text-dark-400 text-sm mb-2">Rust (also what an on-chain verifier does):</p>
                  <pre className="bg-dark-950 rounded-lg p-4 overflow-x-auto">
                    <code className="text-xs text-dark-300">{`use ed25519_dalek::{Signature, Verifier, VerifyingKey};

let key_bytes: [u8; 32] = bs58::decode(pinned_pubkey).into_vec()?.try_into().unwrap();
let vk = VerifyingKey::from_bytes(&key_bytes)?;
let sig = Signature::from_bytes(&base64_decode(signature_b64)?.try_into().unwrap());

vk.verify(payload.as_bytes(), &sig)?;   // payload as received, byte for byte`}</code>
                  </pre>
                </div>
              </details>

              <h3 className="text-lg font-semibold text-white mb-4">Verifying inside a NEAR contract</h3>
              <p className="text-dark-300 mb-4">
                NEAR exposes Ed25519 verification as a host function, so checking the feed on-chain is cheap and needs
                no crypto library. This is what makes the relayer untrusted: anyone may submit the payload, and the
                contract accepts it purely on the signature.
              </p>
              <details className="card mb-6">
                <summary className="cursor-pointer text-white font-semibold">Rust: a receiver anyone can call</summary>
                <pre className="bg-dark-950 rounded-lg p-4 overflow-x-auto mt-4">
                  <code className="text-xs text-dark-300">{`use near_sdk::{env, near, require, store::LookupMap};
use near_sdk::base64::{engine::general_purpose::STANDARD, Engine};
use near_sdk::json_types::I64;
use near_sdk::serde::Deserialize;
use std::collections::HashMap;

/// The pinned feed key: base58 of "ed25519:..." decoded to 32 raw bytes.
const FEED_PUBKEY: [u8; 32] = [/* 32 bytes */];
const MAX_AGE_SECS: u64 = 120;

#[derive(Deserialize)]
#[serde(crate = "near_sdk::serde")]
struct Entry {
    price: I64,         // sent as a decimal string; I64 is exactly that convention
    expo: i32,
    publish_time: i64,
}

#[near]
impl Contract {
    /// Permissionless: the caller is untrusted, the signature is what counts.
    pub fn submit_prices(&mut self, payload: String, signature: String) {
        // 1. Verify over the EXACT bytes received — never re-serialize first.
        let sig: [u8; 64] = STANDARD.decode(&signature).expect("bad base64")
            .try_into().expect("signature must be 64 bytes");
        require!(
            env::ed25519_verify(&sig, payload.as_bytes(), &FEED_PUBKEY),
            "invalid feed signature"
        );

        // 2. Only after it verifies is the payload safe to parse.
        let entries: HashMap<String, Entry> =
            near_sdk::serde_json::from_str(&payload).expect("malformed payload");

        let now = env::block_timestamp() / 1_000_000_000;
        for (asset, e) in entries {
            let published = e.publish_time as u64;
            require!(now.saturating_sub(published) <= MAX_AGE_SECS, "price too old");

            // 3. Replay guard: a signed payload stays valid forever, so never
            //    overwrite with anything that is not strictly newer. Skip, do not
            //    fail: an equal stamp is the normal result of polling inside one
            //    refresh cycle, and a transaction that panics on it is wasted gas.
            if let Some(prev) = self.prices.get(&asset) {
                if published <= prev.publish_time {
                    continue;
                }
            }

            self.prices.insert(asset, StoredPrice { price: e.price.0, expo: e.expo, publish_time: published });
        }
    }
}`}</code>
                </pre>
                <p className="text-dark-400 text-sm mt-3">
                  Three checks carry the whole design: the signature (authenticity), the age bound (freshness), and the
                  strictly-newer <code className="text-primary">publish_time</code> rule (replay). Drop any of them and
                  an old but validly signed payload can be replayed later. The third one skips rather than fails: a
                  repeated stamp is what a poll inside one refresh cycle legitimately returns.
                </p>
              </details>

              <details className="card mb-6">
                <summary className="cursor-pointer text-white font-semibold">Borsh format (for on-chain verification)</summary>
                <p className="text-dark-400 text-sm mt-3 mb-3">
                  Pass <code className="text-primary">&quot;sig_format&quot;: &quot;borsh&quot;</code>. The{' '}
                  <code className="text-primary">payload</code> becomes base64 of the borsh bytes, and the signature is
                  over the <strong className="text-white">decoded</strong> bytes. Layout of{' '}
                  <code className="text-primary">BTreeMap&lt;String, PriceEntry&gt;</code>, keys ascending:
                </p>
                <pre className="bg-dark-950 rounded-lg p-4 overflow-x-auto">
                  <code className="text-xs text-dark-300">{`u32  entry_count            (little-endian)
repeated per entry:
  u32  key_len               (little-endian)
  ..   key bytes             (UTF-8)
  i64  price                 (little-endian)
  i32  expo                  (little-endian)
  i64  publish_time          (little-endian)`}</code>
                </pre>
                <p className="text-dark-400 text-sm mt-3">
                  Use <code className="text-primary">near_sdk::json_types::I64</code> for{' '}
                  <code className="text-primary">price</code> in both formats. Under borsh it is interchangeable with
                  a plain <code className="text-primary">i64</code> — it is a newtype, so the same eight
                  little-endian bytes are written either way. Under JSON it is <strong className="text-white">not
                  optional</strong>: <code className="text-primary">price</code> is a decimal string, and a bare{' '}
                  <code className="text-primary">i64</code> field fails to deserialize with{' '}
                  <code className="text-primary">invalid type: string</code>. Declaring it as{' '}
                  <code className="text-primary">I64</code> covers both and needs no manual parse.
                </p>
              </details>

              <h3 id="publish-time" className="text-lg font-semibold text-white mb-4">
                What <code className="text-primary">publish_time</code> is, and why two polls can return the same one
              </h3>
              <div className="card mb-6">
                <ul className="list-disc list-inside text-dark-300 space-y-3">
                  <li>
                    <strong className="text-white">It is an observation time, not a response time.</strong> Every
                    venue is stamped when the enclave read it. The price is aggregated over the venues inside your{' '}
                    <code className="text-primary">max_age_secs</code> window, and{' '}
                    <code className="text-primary">publish_time</code> is the oldest of them: an aggregate is no
                    fresher than its stalest input, and that is the number that gets signed.
                  </li>
                  <li>
                    <strong className="text-white">The cache is refreshed in cycles, not per request.</strong> A
                    scheduler refreshes the asset set continuously. The cadence is operator configuration, not part of
                    the feed contract, and a cycle occasionally runs longer when a venue answers slowly. A request the
                    cache can satisfy never touches a venue.
                  </li>
                  <li>
                    <strong className="text-white">So two polls inside one cycle return the same signed price with the
                    same <code className="text-primary">publish_time</code>.</strong> Nothing was observed in between,
                    and the value is still inside the window you asked for. Treat it as &quot;no new data&quot;. Only
                    its age can make it wrong, and the age bound is your check.
                  </li>
                  <li>
                    <strong className="text-white">Assets in one response may carry different stamps.</strong> A
                    refresh writes the assets one at a time over a few seconds, so a request that lands during a
                    refresh sees the new stamp on the assets already written and the previous one on the rest. Each is
                    inside your window and signed with its own stamp.
                  </li>
                  <li>
                    <strong className="text-white">If you push prices on-chain,</strong> drop every asset whose{' '}
                    <code className="text-primary">publish_time</code> equals the one you last submitted before sending
                    the transaction, and have the contract skip, not fail, a stamp that is not strictly newer — as the
                    receiver above does. A hard monotonic check turns every repeated poll into a rejected transaction.
                  </li>
                  <li>
                    <strong className="text-white">Need fresher observations than the cache gives you?</strong> Narrow{' '}
                    <code className="text-primary">max_age_secs</code>. When the cache cannot satisfy the window the
                    enclave fetches live from the allowed venues inside the call, at a few seconds of latency per
                    request. The window is the guarantee; a refresh every N seconds is not.
                  </li>
                </ul>
              </div>

              <h3 className="text-lg font-semibold text-white mb-4">Running it in production</h3>
              <ol className="list-decimal list-inside text-dark-300 space-y-3 mb-6">
                <li>
                  <strong className="text-white">Pin the public key</strong> from step 1 and verify its attestation
                  once. Treat a changed key as a failure, not as something to auto-accept.
                </li>
                <li>
                  <strong className="text-white">Poll on your own cadence.</strong> Prices are refreshed continuously,
                  so a poll normally returns the cached value. Ask for the window you actually need —{' '}
                  <code className="text-primary">max_age_secs: 40</code> means &quot;built only from venues seen in the
                  last 40 seconds&quot;. Narrowing it below our refresh cadence is allowed and simply makes the call
                  fetch fresh, which costs a few seconds of latency; widening it lets the slower venues (Pyth and
                  Chainlink, which run on their own wider cycle) contribute as well.
                  <br />
                  <span className="text-dark-400">
                    There is a practical floor: a fresh fetch takes several seconds, and the prices it produces are
                    already that old by the time the request is answered, so a very tight window fails whenever the
                    cache cannot serve it. Do not guess where that floor is — the answer tells you.{' '}
                    <code className="text-primary">publish_time</code> is the oldest source that contributed, so
                    measure a few responses and set your window from what you observe. The assets under the heaviest
                    use are refreshed on the fastest cycle.
                  </span>
                </li>
                <li>
                  <strong className="text-white">Verify first, parse second</strong> — over the raw payload bytes.
                </li>
                <li>
                  <strong className="text-white">Enforce freshness yourself.</strong> Check{' '}
                  <code className="text-primary">publish_time</code> against your own bound; do not rely only on our
                  server-side check.
                </li>
                <li>
                  <strong className="text-white">Never store a non-increasing timestamp</strong> if you write prices to
                  a contract — that is what stops an old signed payload from being replayed. Skip it rather than fail
                  on it: repeated polls within one refresh cycle legitimately return the same{' '}
                  <code className="text-primary">publish_time</code>, and that is &quot;no new data&quot;, not an
                  error — see <a href="#publish-time" className="text-primary hover:underline">above</a>.
                </li>
                <li>
                  <strong className="text-white">Fail closed.</strong> A failed request or signature means no price —
                  never fall back to an unsigned or stale value.
                </li>
              </ol>

              <div className="card">
                <p className="text-dark-400 text-sm">
                  <strong className="text-white">Access.</strong> Calls need a payment key authorised for this project.
                  If you already have an OutLayer payment key, ask for this project to be allowed on it; otherwise we
                  can issue one. The request is compute-only — it never touches the chain and costs no gas.
                </p>
              </div>
            </section>

            {/* Native Pyth Interface */}
            <section id="pyth-native" className="mb-16">
              <h2 className="text-2xl font-bold text-white mb-6">Native Pyth Interface</h2>
              <p className="text-dark-300 mb-6">
                <code className="text-primary">price-oracle.near</code> implements Pyth-compatible view methods natively.
                DeFi contracts using <code>pyth-oracle.near</code> can migrate by changing one contract address — no code changes needed.
              </p>

              <div className="card border-green-500/30 bg-green-500/5 mb-6">
                <div className="flex items-start gap-3">
                  <div className="text-green-400 text-xl">✓</div>
                  <div>
                    <h3 className="text-lg font-semibold text-white mb-2">No refresh_prices Needed</h3>
                    <p className="text-dark-400 text-sm">
                      Unlike the separate Pyth wrapper contract, the native interface reads directly from contract
                      state, so there is no refresh call to make. That state is written on its own slow, gas-paying
                      cycle — read the timestamp it returns and enforce your own staleness bound, exactly as you
                      would against any on-chain feed.
                    </p>
                  </div>
                </div>
              </div>

              <h3 className="text-lg font-semibold text-white mb-4">View Methods (free — check the timestamp)</h3>
              <div className="overflow-x-auto mb-8">
                <table className="w-full text-left text-sm">
                  <thead>
                    <tr className="border-b border-dark-700">
                      <th className="py-3 px-4 text-dark-300">Method</th>
                      <th className="py-3 px-4 text-dark-300">Description</th>
                    </tr>
                  </thead>
                  <tbody className="text-dark-400">
                    <tr className="border-b border-dark-800">
                      <td className="py-3 px-4"><code className="text-primary">get_price(price_identifier)</code></td>
                      <td className="py-3 px-4">Latest price with staleness check</td>
                    </tr>
                    <tr className="border-b border-dark-800">
                      <td className="py-3 px-4"><code className="text-primary">get_price_unsafe(price_identifier)</code></td>
                      <td className="py-3 px-4">Latest price without staleness check</td>
                    </tr>
                    <tr className="border-b border-dark-800">
                      <td className="py-3 px-4"><code className="text-primary">get_price_no_older_than(price_id, age)</code></td>
                      <td className="py-3 px-4">Price only if published within <code>age</code> seconds</td>
                    </tr>
                    <tr className="border-b border-dark-800">
                      <td className="py-3 px-4"><code className="text-primary">get_ema_price(price_id)</code></td>
                      <td className="py-3 px-4">EMA price with staleness check</td>
                    </tr>
                    <tr className="border-b border-dark-800">
                      <td className="py-3 px-4"><code className="text-primary">get_ema_price_unsafe(price_id)</code></td>
                      <td className="py-3 px-4">EMA price without staleness check</td>
                    </tr>
                    <tr className="border-b border-dark-800">
                      <td className="py-3 px-4"><code className="text-primary">list_prices(price_ids)</code></td>
                      <td className="py-3 px-4">Batch: multiple feeds at once</td>
                    </tr>
                    <tr className="border-b border-dark-800">
                      <td className="py-3 px-4"><code className="text-primary">price_feed_exists(price_identifier)</code></td>
                      <td className="py-3 px-4">Check if feed is configured</td>
                    </tr>
                    <tr className="border-b border-dark-800">
                      <td className="py-3 px-4"><code className="text-primary">get_update_fee_estimate(data)</code></td>
                      <td className="py-3 px-4">Returns 1 yoctoNEAR (no update needed)</td>
                    </tr>
                  </tbody>
                </table>
              </div>

              <h3 className="text-lg font-semibold text-white mb-4">Migration from Pyth</h3>
              <pre className="bg-dark-900 rounded-lg p-4 overflow-x-auto mb-6">
                <code className="text-blue-400">
{`// Before (Pyth)
const ORACLE: &str = "pyth-oracle.near";

// After (Oracle Example) — no other changes needed!
const ORACLE: &str = "price-oracle.near";`}
                </code>
              </pre>

              <h3 className="text-lg font-semibold text-white mb-4">Response Format</h3>
              <pre className="bg-dark-900 rounded-lg p-4 overflow-x-auto">
                <code className="text-blue-400">
{`// PythPrice format (same as pyth-oracle.near)
{
  "price": 525000000,      // price * 10^|expo|
  "conf": 0,               // confidence (always 0 for Oracle Example)
  "expo": -8,              // exponent: actual_price = price * 10^expo
  "publish_time": 1706900000  // unix timestamp (seconds)
}
// Example: price=525000000, expo=-8 → $5.25`}
                </code>
              </pre>
            </section>

            {/* Direct OutLayer Integration */}
            <section id="direct-outlayer" className="mb-16">
              <h2 className="text-2xl font-bold text-white mb-6">Direct OutLayer Integration</h2>
              <p className="text-dark-300 mb-6">
                You don&apos;t need to use <code className="text-primary">price-oracle.near</code> at all.
                Your contract can call OutLayer directly to fetch prices or any custom data from TEE.
              </p>

              <div className="card border-purple-500/30 bg-purple-500/5 mb-6">
                <div className="flex items-start gap-3">
                  <div className="text-purple-400 text-xl">★</div>
                  <div>
                    <h3 className="text-lg font-semibold text-white mb-2">Why Go Direct?</h3>
                    <ul className="text-dark-400 text-sm space-y-2">
                      <li>No intermediary contracts — full control over the flow</li>
                      <li>Custom WASI workers — fetch any data you need</li>
                      <li>Lower gas costs — one less cross-contract call</li>
                      <li>Your contract owns the entire integration</li>
                    </ul>
                  </div>
                </div>
              </div>

              <h3 className="text-lg font-semibold text-white mb-4">Step 1: Call OutLayer request_execution</h3>
              <pre className="bg-dark-900 rounded-lg p-4 overflow-x-auto mb-6">
                <code className="text-blue-400">
{`use near_sdk::{ext_contract, AccountId, NearToken, Promise, serde_json};

#[ext_contract(ext_outlayer)]
pub trait OutLayer {
    fn request_execution(
        &mut self,
        execution_source: serde_json::Value,
        resource_limits: Option<serde_json::Value>,
        input_data: Option<String>,
        secrets_ref: Option<serde_json::Value>,
        response_format: Option<String>,
        payer_account_id: Option<AccountId>,
        callback_receiver_id: Option<AccountId>,
    ) -> Promise;
}

impl Contract {
    pub fn fetch_price(&mut self, token_id: String) -> Promise {
        // Use the deployed price oracle project
        // Mainnet: "price-oracle.near/price-oracle"
        // Testnet: "price-oracle.testnet/price-oracle"
        let execution_source = serde_json::json!({
            "Project": {
                "project_id": "price-oracle.near/price-oracle"
            }
        });

        // Resource limits (recommended)
        let resource_limits = serde_json::json!({
            "max_instructions": 10000000000_u64,
            "max_memory_mb": 128,
            "max_execution_seconds": 60
        });

        // Input data for the WASI worker (see OracleCommand in types.rs)
        let input_data = serde_json::json!({
            "command": "get_prices",
            "tokens": [token_id]
        }).to_string();

        // Call OutLayer directly
        ext_outlayer::ext("outlayer.near".parse().unwrap())
            .with_attached_deposit(NearToken::from_millinear(10)) // 0.01 NEAR
            .with_unused_gas_weight(1)
            .request_execution(
                execution_source,
                Some(resource_limits),          // resource limits
                Some(input_data),               // your request
                None,                           // no secrets needed
                Some("json".to_string()),       // response format
                Some(env::predecessor_account_id()), // payer
                Some(env::current_account_id()), // callback receiver
            )
    }
}`}
                </code>
              </pre>

              <h3 className="text-lg font-semibold text-white mb-4">Step 2: Handle the Callback</h3>
              <pre className="bg-dark-900 rounded-lg p-4 overflow-x-auto mb-6">
                <code className="text-blue-400">
{`// OutLayer calls this method with the TEE result
#[private] // Only callable by self (via promise)
pub fn on_outlayer_result(
    &mut self,
    #[callback_result] result: Result<serde_json::Value, near_sdk::PromiseError>,
) {
    match result {
        Ok(data) => {
            // Parse the price data from TEE response
            if let Some(prices) = data.get("prices") {
                // Process your prices here
                log!("Got prices from TEE: {:?}", prices);
            }
        }
        Err(e) => {
            log!("OutLayer call failed: {:?}", e);
        }
    }
}`}
                </code>
              </pre>

              <h3 className="text-lg font-semibold text-white mb-4">Architecture: Direct vs Via Oracle Contract</h3>
              <div className="card bg-dark-900 mb-6">
                <pre className="text-sm text-dark-300 overflow-x-auto">
{`Via price-oracle.near (simpler):
Your Contract → price-oracle.near → OutLayer → TEE → price-oracle.near → Your Contract

Direct OutLayer (more control):
Your Contract → OutLayer → TEE → Your Contract

Both are valid! Use price-oracle.near for quick integration,
or go direct for full customization.`}
                </pre>
              </div>

              <div className="card border-yellow-500/30 bg-yellow-500/5">
                <div className="flex items-start gap-3">
                  <div className="text-yellow-400 text-xl">⚠️</div>
                  <div>
                    <h3 className="text-lg font-semibold text-white mb-2">Important Notes</h3>
                    <ul className="text-dark-400 text-sm space-y-2">
                      <li>You need to deploy your own WASI worker or use an existing one (like the price oracle WASI)</li>
                      <li>For price fetching, it&apos;s easier to use <code className="text-primary">price-oracle.near</code> — it handles WASI configuration for you</li>
                      <li>Direct integration is best for custom data sources or when you need full control</li>
                      <li>See{' '}
                        <a href="https://github.com/out-layer/oracle-example/tree/main/contract" target="_blank" rel="noopener noreferrer" className="text-primary hover:underline">
                          price-oracle contract source
                        </a>
                        {' '}for a complete example
                      </li>
                    </ul>
                  </div>
                </div>
              </div>
            </section>

            {/* Price Oracle Contract */}
            <section id="price-oracle" className="mb-16">
              <h2 className="text-2xl font-bold text-white mb-6">Price Oracle Contract</h2>

              <div className="card border-blue-500/30 bg-blue-500/5 mb-6">
                <div className="flex items-start gap-3">
                  <div className="text-blue-400 text-xl">i</div>
                  <div>
                    <h3 className="text-lg font-semibold text-white mb-2">Free reads, but verify the age</h3>
                    <p className="text-dark-400 text-sm mb-3">
                      <code className="text-primary">price-oracle.near</code> is written to by TEE workers on a
                      slow, gas-paying cycle, so <code className="text-primary">get_price_data</code> costs you
                      nothing but carries no promise of freshness. Compare the timestamp it returns against your
                      own bound and fail closed if it is too old.
                    </p>
                    <p className="text-dark-400 text-sm">
                      You can also integrate with OutLayer directly from your own contract (see Direct OutLayer Integration section above).
                    </p>
                  </div>
                </div>
              </div>

              <p className="text-dark-300 mb-6">
                Contract address: <code className="text-primary">price-oracle.near</code>
              </p>
              <p className="text-dark-400 text-sm mb-6">
                This contract recreates the interface (with additions) of the original{' '}
                <a href="https://github.com/NearDeFi/price-oracle" target="_blank" rel="noopener noreferrer" className="text-primary hover:underline">
                  NEAR Native Price Oracle
                </a>{' '}
                — existing integrations can migrate with minimal changes.
              </p>

              <h3 className="text-lg font-semibold text-white mb-4">View Methods (free)</h3>
              <div className="card border-blue-500/30 bg-blue-500/5 mb-4">
                <div className="flex items-start gap-3">
                  <div className="text-blue-400 text-lg">i</div>
                  <p className="text-dark-400 text-sm">
                    On-chain state is written on a slow cycle, so <code className="text-primary">get_price_data</code>{' '}
                    is free but may be stale — check its timestamp. Call{' '}
                    <code className="text-primary">request_price_data</code> when you need a price fetched for
                    that specific call.
                  </p>
                </div>
              </div>
              <div className="overflow-x-auto mb-8">
                <table className="w-full text-left text-sm">
                  <thead>
                    <tr className="border-b border-dark-700">
                      <th className="py-3 px-4 text-dark-300">Method</th>
                      <th className="py-3 px-4 text-dark-300">Arguments</th>
                      <th className="py-3 px-4 text-dark-300">Description</th>
                    </tr>
                  </thead>
                  <tbody className="text-dark-400">
                    <tr className="border-b border-dark-800">
                      <td className="py-3 px-4"><code className="text-primary">get_price_data</code></td>
                      <td className="py-3 px-4"><code>asset_ids?: string[]</code></td>
                      <td className="py-3 px-4">Get cached prices. Always returns a PriceData object; per-asset price is null when stale/unavailable</td>
                    </tr>
                    <tr className="border-b border-dark-800">
                      <td className="py-3 px-4"><code className="text-primary">can_subsidize_outlayer_calls</code></td>
                      <td className="py-3 px-4">—</td>
                      <td className="py-3 px-4">Check if contract pays for calls</td>
                    </tr>
                    <tr className="border-b border-dark-800">
                      <td className="py-3 px-4"><code className="text-primary">get_oracle_price_data</code></td>
                      <td className="py-3 px-4"><code>account_id, asset_ids?</code></td>
                      <td className="py-3 px-4">Get prices from specific oracle</td>
                    </tr>
                  </tbody>
                </table>
              </div>

              <h3 className="text-lg font-semibold text-white mb-4">Call Methods (require deposit)</h3>
              <div className="overflow-x-auto mb-8">
                <table className="w-full text-left text-sm">
                  <thead>
                    <tr className="border-b border-dark-700">
                      <th className="py-3 px-4 text-dark-300">Method</th>
                      <th className="py-3 px-4 text-dark-300">Deposit</th>
                      <th className="py-3 px-4 text-dark-300">Description</th>
                    </tr>
                  </thead>
                  <tbody className="text-dark-400">
                    <tr className="border-b border-dark-800">
                      <td className="py-3 px-4"><code className="text-primary">request_price_data</code></td>
                      <td className="py-3 px-4 text-yellow-400">0.01+ NEAR</td>
                      <td className="py-3 px-4">Get prices directly</td>
                    </tr>
                    <tr className="border-b border-dark-800">
                      <td className="py-3 px-4"><code className="text-primary">oracle_call</code></td>
                      <td className="py-3 px-4 text-yellow-400">0.01+ NEAR</td>
                      <td className="py-3 px-4">Get prices with callback</td>
                    </tr>
                    <tr className="border-b border-dark-800">
                      <td className="py-3 px-4"><code className="text-primary">request_custom_data</code></td>
                      <td className="py-3 px-4 text-yellow-400">0.01+ NEAR</td>
                      <td className="py-3 px-4">Fetch custom external data</td>
                    </tr>
                    <tr className="border-b border-dark-800">
                      <td className="py-3 px-4"><code className="text-primary">custom_call</code></td>
                      <td className="py-3 px-4 text-yellow-400">0.01+ NEAR</td>
                      <td className="py-3 px-4">Custom data with callback</td>
                    </tr>
                  </tbody>
                </table>
              </div>

              <h3 className="text-lg font-semibold text-white mb-4">Data Types</h3>
              <pre className="bg-dark-900 rounded-lg p-4 overflow-x-auto mb-6">
                <code className="text-blue-400">
{`// Price format: multiplier / 10^decimals = USD
struct Price {
    multiplier: u128,  // e.g., 500000000 for $5.00
    decimals: u8,      // usually 8
}

struct PriceData {
    timestamp: u64,              // nanoseconds
    recency_duration_sec: u32,   // max age for "fresh" prices
    prices: Vec<AssetOptionalPrice>,
}

struct AssetOptionalPrice {
    asset_id: String,
    price: Option<Price>,  // None if stale/unavailable
}`}
                </code>
              </pre>

              <h3 className="text-lg font-semibold text-white mb-4">Callback Interface</h3>
              <pre className="bg-dark-900 rounded-lg p-4 overflow-x-auto">
                <code className="text-blue-400">
{`// Your contract must implement this for oracle_call
pub fn oracle_on_call(
    &mut self,
    sender_id: AccountId,
    data: PriceData,
    msg: String,
) {
    // Verify caller is the oracle
    assert_eq!(
        env::predecessor_account_id(),
        "price-oracle.near".parse::<AccountId>().unwrap(),
        "Only oracle can call"
    );
    // Process prices...
}`}
                </code>
              </pre>
            </section>

            {/* Wrapper Example */}
            <section id="wrapper-example" className="mb-16">
              <h2 className="text-2xl font-bold text-white mb-6">Integration Example: Wrapper Contract</h2>
              <p className="text-dark-300 mb-6">
                A complete example showing how to integrate the oracle with the full callback cycle.
                The wrapper contract self-funds oracle calls and handles callbacks internally.
              </p>
              <p className="text-dark-300 mb-6">
                Contract: <code className="text-primary">price-oracle-wrapper.near</code> |{' '}
                <a
                  href="https://github.com/out-layer/oracle-example/tree/main/wrapper-contract"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-primary hover:underline"
                >
                  Source on GitHub
                </a>
              </p>

              <h3 className="text-lg font-semibold text-white mb-4">How It Works</h3>
              <div className="card bg-dark-900 mb-6">
                <pre className="text-sm text-dark-300 overflow-x-auto">
{`User calls get_price() on Wrapper
        │
        ▼
Wrapper calls oracle_call() with SELF as receiver_id
(self-funded: 0.02 NEAR attached automatically)
        │
        ▼
Oracle processes request via OutLayer TEE
        │
        ▼
Oracle calls oracle_on_call() on Wrapper
        │
        ▼
Wrapper receives prices in callback, processes them`}
                </pre>
              </div>

              <h3 className="text-lg font-semibold text-white mb-4">Key Pattern: Self-Funding Calls</h3>
              <pre className="bg-dark-900 rounded-lg p-4 overflow-x-auto mb-6">
                <code className="text-blue-400">
{`// Wrapper pays for oracle calls itself - users don't need to attach deposits
pub fn get_price(&mut self, token_id: String) -> Promise {
    ext_oracle::ext(self.oracle_contract_id.clone())
        .with_attached_deposit(NearToken::from_millinear(20)) // 0.02 NEAR
        .with_unused_gas_weight(1)
        .oracle_call(
            env::current_account_id(), // callback comes back HERE
            Some(vec![token_id]),
            String::new(),
            None,
        )
}

// Callback handler - called by oracle with price data
pub fn oracle_on_call(
    &mut self,
    sender_id: AccountId,
    data: PriceData,
    msg: String,
) -> Option<Price> {
    // IMPORTANT: verify caller is the oracle!
    assert_eq!(env::predecessor_account_id(), self.oracle_contract_id);

    // Extract price from data
    if let Some(asset) = data.prices.first() {
        return asset.price.clone();
    }
    None
}`}
                </code>
              </pre>

              <div className="card border-green-500/30 bg-green-500/5 mb-6">
                <div className="flex items-start gap-3">
                  <div className="text-green-400 text-xl">✓</div>
                  <div>
                    <h3 className="text-lg font-semibold text-white mb-2">Why This Pattern?</h3>
                    <ul className="text-dark-400 text-sm space-y-2">
                      <li><strong className="text-white">Self-funding:</strong> Users call your contract without deposits — your contract pays for oracle calls</li>
                      <li><strong className="text-white">Full cycle:</strong> Request → TEE → Callback all handled in one user transaction</li>
                      <li><strong className="text-white">Security:</strong> Always verify <code className="text-primary">predecessor_account_id</code> in callbacks</li>
                      <li><strong className="text-white">Context:</strong> Use the <code className="text-primary">msg</code> field to pass context through async chain</li>
                    </ul>
                  </div>
                </div>
              </div>

              <div className="card border-blue-500/30 bg-blue-500/5">
                <div className="flex items-start gap-3">
                  <div className="text-blue-400 text-xl">i</div>
                  <div>
                    <p className="text-dark-400 text-sm">
                      <strong className="text-white">All example contracts are optional!</strong> The contracts we provide (<code className="text-primary">price-oracle.near</code>, <code className="text-primary">price-oracle-wrapper.near</code>, etc.) are just examples.
                      You can integrate with OutLayer directly from your own contract — see the next section.
                    </p>
                  </div>
                </div>
              </div>
            </section>

            {/* Legacy Pyth Wrapper — redirect to native */}
            <section id="pyth-wrapper" className="mb-16">
              <h2 className="text-2xl font-bold text-white mb-6">Legacy Pyth Wrapper</h2>
              <div className="card border-blue-500/30 bg-blue-500/5 mb-6">
                <div className="flex items-start gap-3">
                  <div className="text-blue-400 text-xl">i</div>
                  <div>
                    <p className="text-dark-400 text-sm mb-3">
                      <strong className="text-white">Pyth-compatible methods are now built into <code className="text-primary">price-oracle.near</code> directly.</strong>{' '}
                      The separate <code className="text-dark-300">price-oracle-pyth.near</code> wrapper is no longer needed.
                    </p>
                    <p className="text-dark-400 text-sm">
                      See the{' '}
                      <button onClick={() => scrollToSection('pyth-native')} className="text-primary hover:underline">
                        Native Pyth Interface
                      </button>{' '}
                      section for migration instructions. Simply change your contract address to <code className="text-primary">price-oracle.near</code> — all Pyth view methods work natively, with always-fresh prices (no <code className="text-dark-300">refresh_prices</code> call needed).
                    </p>
                  </div>
                </div>
              </div>
            </section>

            {/* Custom Data */}
            <section id="custom-data" className="mb-16">
              <h2 className="text-2xl font-bold text-white mb-6">Custom Data Sources</h2>
              <p className="text-dark-300 mb-6">
                Fetch data from any HTTP API via TEE using <code>request_custom_data</code> or <code>custom_call</code>.
              </p>

              <h3 className="text-lg font-semibold text-white mb-4">Request Format</h3>
              <pre className="bg-dark-900 rounded-lg p-4 overflow-x-auto mb-6">
                <code className="text-blue-400">
{`{
  "custom_data_request": [
    {
      "id": "my_data",           // Identifier for the result
      "token_id": "",            // Optional token identifier
      "source": {
        "custom": {
          "url": "https://api.example.com/data",
          "json_path": "result.value",   // Dot notation path
          "value_type": "number",        // "number", "string", "boolean"
          "method": "GET",               // "GET" or "POST"
          "headers": []                  // Optional headers
        }
      }
    }
  ]
}`}
                </code>
              </pre>

              <h3 className="text-lg font-semibold text-white mb-4">Examples</h3>
              <div className="space-y-4">
                <div className="card">
                  <h4 className="font-medium text-white mb-2">Steam Game Price</h4>
                  <pre className="bg-dark-950 rounded p-3 text-xs overflow-x-auto">
                    <code className="text-green-400">
{`{
  "url": "https://store.steampowered.com/api/appdetails?appids=1245620",
  "json_path": "1245620.data.price_overview.final_formatted"
}`}
                    </code>
                  </pre>
                </div>
                <div className="card">
                  <h4 className="font-medium text-white mb-2">Account NFTs (FastNEAR)</h4>
                  <pre className="bg-dark-950 rounded p-3 text-xs overflow-x-auto">
                    <code className="text-green-400">
{`{
  "url": "https://api.fastnear.com/v1/account/root.near/nft",
  "json_path": "tokens"
}`}
                    </code>
                  </pre>
                </div>
                <div className="card">
                  <h4 className="font-medium text-white mb-2">Weather Data</h4>
                  <pre className="bg-dark-950 rounded p-3 text-xs overflow-x-auto">
                    <code className="text-green-400">
{`{
  "url": "https://api.open-meteo.com/v1/forecast?latitude=40.71&longitude=-74.00&current_weather=true",
  "json_path": "current_weather.temperature"
}`}
                    </code>
                  </pre>
                </div>
              </div>
            </section>

            {/* Code Examples */}
            <section id="code-examples" className="mb-16">
              <h2 className="text-2xl font-bold text-white mb-6">Code Examples</h2>

              <h3 className="text-lg font-semibold text-white mb-4">Rust Integration</h3>
              <pre className="bg-dark-900 rounded-lg p-4 overflow-x-auto mb-6">
                <code className="text-blue-400">
{`use near_sdk::{ext_contract, AccountId, Gas, NearToken, Promise};

#[ext_contract(ext_oracle)]
pub trait Oracle {
    fn oracle_call(
        &mut self,
        receiver_id: AccountId,
        asset_ids: Option<Vec<String>>,
        msg: String,
        resource_limits: Option<serde_json::Value>,
    ) -> Promise;
}

impl Contract {
    pub fn get_prices_with_callback(&self) -> Promise {
        ext_oracle::ext("price-oracle.near".parse().unwrap())
            .with_attached_deposit(NearToken::from_millinear(20))
            .with_static_gas(Gas::from_tgas(150))
            .oracle_call(
                env::current_account_id(),
                Some(vec!["wrap.near".to_string()]),
                "swap".to_string(),
                None,
            )
    }
}`}
                </code>
              </pre>

              <h3 className="text-lg font-semibold text-white mb-4">JavaScript Integration</h3>
              <pre className="bg-dark-900 rounded-lg p-4 overflow-x-auto">
                <code className="text-blue-400">
{`import { connect, Contract } from 'near-api-js';

const oracle = new Contract(account, 'price-oracle.near', {
  viewMethods: ['get_price_data'],
  changeMethods: ['request_price_data', 'oracle_call'],
});

// View cached prices (free)
const cached = await oracle.get_price_data({
  asset_ids: ['wrap.near', 'eth.bridge.near'],
});

// Convert price
const price = cached.prices[0].price;
const usd = Number(price.multiplier) / Math.pow(10, price.decimals);
console.log(\`NEAR = $\${usd}\`);`}
                </code>
              </pre>
            </section>

            {/* Deposits */}
            <section id="deposits" className="mb-16">
              <h2 className="text-2xl font-bold text-white mb-6">Deposit Requirements</h2>

              <div className="overflow-x-auto">
                <table className="w-full text-left text-sm">
                  <thead>
                    <tr className="border-b border-dark-700">
                      <th className="py-3 px-4 text-dark-300">Method</th>
                      <th className="py-3 px-4 text-dark-300">Fresh Cache</th>
                      <th className="py-3 px-4 text-dark-300">Stale (OutLayer)</th>
                      <th className="py-3 px-4 text-dark-300">Subsidized</th>
                    </tr>
                  </thead>
                  <tbody className="text-dark-400">
                    <tr className="border-b border-dark-800">
                      <td className="py-3 px-4"><code>get_price_data</code></td>
                      <td className="py-3 px-4 text-green-400">Free</td>
                      <td className="py-3 px-4">N/A</td>
                      <td className="py-3 px-4">N/A</td>
                    </tr>
                    <tr className="border-b border-dark-800">
                      <td className="py-3 px-4"><code>request_price_data</code></td>
                      <td className="py-3 px-4 text-green-400">Free</td>
                      <td className="py-3 px-4 text-yellow-400">0.01+ NEAR</td>
                      <td className="py-3 px-4 text-green-400">Free</td>
                    </tr>
                    <tr className="border-b border-dark-800">
                      <td className="py-3 px-4"><code>oracle_call</code></td>
                      <td className="py-3 px-4 text-dark-500">1 yoctoNEAR</td>
                      <td className="py-3 px-4 text-yellow-400">0.01+ NEAR</td>
                      <td className="py-3 px-4 text-green-400">Free</td>
                    </tr>
                    <tr className="border-b border-dark-800">
                      <td className="py-3 px-4"><code>request_custom_data</code></td>
                      <td className="py-3 px-4">N/A</td>
                      <td className="py-3 px-4 text-yellow-400">0.01+ NEAR</td>
                      <td className="py-3 px-4 text-green-400">Free</td>
                    </tr>
                  </tbody>
                </table>
              </div>

              <div className="mt-6 card border-green-500/30">
                <h4 className="font-medium text-white mb-2">Subsidized Mode</h4>
                <p className="text-dark-400 text-sm">
                  When contract has {'>'}20 NEAR and subsidy is enabled, all OutLayer calls are free.
                  Check with <code>can_subsidize_outlayer_calls()</code>.
                </p>
              </div>
            </section>

            {/* Try Playground */}
            <section className="mb-16">
              <div className="card bg-gradient-to-r from-primary/20 to-secondary/20 border-primary/30">
                <h2 className="text-2xl font-bold text-white mb-4">Try It Out</h2>
                <p className="text-dark-300 mb-6">
                  Use the interactive playground to test oracle methods without writing code.
                </p>
                <Link href="/playground" className="btn btn-primary">
                  Open Playground
                </Link>
              </div>
            </section>
          </main>
        </div>
      </div>
    </div>
  );
}
