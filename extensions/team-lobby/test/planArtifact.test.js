'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const { formatPlanArtifact } = require('../dist/planArtifact.js');

test('plan artifact keeps request, reviewable proposal, and bounded provenance together', () => {
	const artifact = formatPlanArtifact({ request: 'Add a room relay\nKeep it secure', plan: '## Steps\n\n1. Add relay\n2. Verify TLS', model: 'Example Model (Vendor)', task: 'Ship relay | Phase 1', createdAt: '2026-10-21T12:00:00.000Z' });
	assert.match(artifact, /^# Forge Room implementation plan/m);
	assert.match(artifact, /\| Created \| 2026-10-21T12:00:00\.000Z \|/);
	assert.match(artifact, /\| Model \| Example Model \(Vendor\) \|/);
	assert.match(artifact, /\| Room task \| Ship relay \\\| Phase 1 \|/);
	assert.match(artifact, /> Add a room relay\n> Keep it secure/);
	assert.match(artifact, /## Proposed plan\n\n## Steps\n\n1\. Add relay/);
});

test('plan artifact rejects empty and oversized plans', () => {
	assert.equal(formatPlanArtifact({ request: '', plan: '  ', model: 'Model' }), undefined);
	assert.equal(formatPlanArtifact({ request: '', plan: 'x'.repeat(100_001), model: 'Model' }), undefined);
});
