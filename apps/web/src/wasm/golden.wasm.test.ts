import { describe, expect, it } from 'vitest';
import cargoToml from '../../Cargo.toml?raw';

/**
 * The WASM package in src/wasm/pkg (built by `pnpm rust:wasm`, not
 * committed) must come from this workspace: a stale package fails here.
 * The golden geometry cases and the independent references run through the
 * app's own path into it (src/model/geom/golden.test.ts, reference.test.ts:
 * since S3b those functions are facades over the core, docs/adr/0008).
 */

/** The generated package's export this test uses (declared here so `tsc` passes before a WASM build). */
interface Wasm {
  coreVersion(): string;
}

const glue = import.meta.glob<Wasm>('./pkg/kentos_geometry_wasm.js');
const loader = Object.values(glue)[0];

describe.skipIf(!loader)('the WASM build', () => {
  it('is built from this workspace version (a stale package fails here)', async () => {
    const version = /\[workspace\.package\][^[]*?\nversion = "([^"]+)"/.exec(cargoToml)?.[1];
    // The page's core module is started by src/wasm/testSetup.ts; this reads the same glue.
    expect((await loader()).coreVersion()).toBe(version);
  });
});
