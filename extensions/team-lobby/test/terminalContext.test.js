'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const { formatTerminalCaptureForModel, latestTerminalEntryForWorkspace, terminalEntriesForWorkspace, terminalRootAfterExecution } = require('../dist/terminalContext.js');

test('a terminal capture gets a project root only when cwd stays in the same workspace', () => {
	assert.equal(terminalRootAfterExecution('file:///projects/app', 'file:///projects/app'), 'file:///projects/app');
	assert.equal(terminalRootAfterExecution('file:///projects/app', 'file:///projects/other'), undefined);
	assert.equal(terminalRootAfterExecution('file:///projects/app', undefined), undefined);
	assert.equal(terminalRootAfterExecution(undefined, 'file:///projects/app'), undefined);
});

test('multi-root context only exposes terminal output captured inside the selected root', () => {
	const entries = [
		{ id: 'frontend', root: 'file:///projects/frontend', output: 'front-end tests' },
		{ id: 'service', root: 'file:///projects/service', output: 'service tests' },
		{ id: 'unknown', output: 'outside or unreported cwd' }
	];
	assert.deepEqual(terminalEntriesForWorkspace(entries, 'file:///projects/service', 2), [entries[1]]);
	assert.deepEqual(terminalEntriesForWorkspace(entries, 'file:///projects/removed', 2), []);
});

test('single-root workspaces keep the prior explicit terminal selection behavior', () => {
	const entries = [
		{ id: 'rooted', root: 'file:///projects/app' },
		{ id: 'other-root', root: 'file:///projects/other' },
		{ id: 'unknown' }
	];
	assert.deepEqual(terminalEntriesForWorkspace(entries, 'file:///projects/app', 1), [entries[0], entries[2]]);
});

test('latest terminal output is selected only from the active project and ignores empty captures', () => {
	const entries = [
		{ id: 'old', root: 'file:///projects/app', output: 'older test result' },
		{ id: 'other-root', root: 'file:///projects/service', output: 'other project result' },
		{ id: 'empty', root: 'file:///projects/app', output: '' },
		{ id: 'new', root: 'file:///projects/app', output: 'latest test result' },
		{ id: 'unknown', output: 'unknown root' }
	];
	assert.equal(latestTerminalEntryForWorkspace(entries, 'file:///projects/app', 2), entries[3]);
	assert.equal(latestTerminalEntryForWorkspace(entries, 'file:///projects/none', 2), undefined);
	assert.equal(latestTerminalEntryForWorkspace(entries, 'file:///projects/app', 1), entries[4]);
});

test('terminal output formatting redacts command credentials and marks partial or truncated output', () => {
	const result = formatTerminalCaptureForModel({ terminal: 'Test terminal', command: 'npm test --token=secret-value', output: 'pass\r\nline\u0000two', completed: false, truncated: true });
	assert.match(result, /still running; output may be partial/);
	assert.match(result, /capture was truncated/);
	assert.match(result, /--token=\[REDACTED\]/);
	assert.doesNotMatch(result, /secret-value/);
	assert.match(result, /pass\nlinetwo/);
});
