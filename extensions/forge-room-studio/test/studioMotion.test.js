'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

function loadStudio(savedState) {
	const source = fs.readFileSync(path.join(__dirname, '../media/studio.js'), 'utf8');
	const handlers = {};
	const messages = [];
	let persisted = savedState;
	const root = { innerHTML: '', addEventListener: (name, handler) => { handlers[`root:${name}`] = handler; } };
	const window = {
		studioInitial: { themes: [], current: '', previous: false },
		forgeStudio: {
			getState: () => persisted,
			setState: value => { persisted = value; },
			postMessage: value => messages.push(value)
		},
		addEventListener: (name, handler) => { handlers[`window:${name}`] = handler; }
	};
	const document = { getElementById: () => root };
	vm.runInNewContext(source, { window, document });
	return { root, handlers, messages, getPersisted: () => persisted };
}

test('decorative motion is disabled by default and can be enabled from the keyboard accessible control', () => {
	const studio = loadStudio(undefined);
	assert.match(studio.root.innerHTML, /class="studio-shell"/);
	assert.match(studio.root.innerHTML, /aria-pressed="false" title="Decorative motion is off by default/);
	assert.match(studio.root.innerHTML, />Motion off<\/button>/);
	studio.handlers['root:click']({ target: { closest: () => ({ disabled: false, dataset: { action: 'motion' } }) } });
	assert.equal(studio.getPersisted().motionEnabled, true);
	assert.match(studio.root.innerHTML, /class="studio-shell motion-enabled is-entering"/);
	assert.match(studio.root.innerHTML, /aria-pressed="true"/);
	assert.equal(JSON.stringify(studio.messages), JSON.stringify([{ command: 'ready' }]));
});

test('an enabled motion preference survives webview reload and reduced-motion CSS still overrides it', () => {
	const studio = loadStudio({ motionEnabled: true });
	assert.match(studio.root.innerHTML, /class="studio-shell motion-enabled is-entering"/);
	const css = fs.readFileSync(path.join(__dirname, '../media/studio.css'), 'utf8');
	assert.match(css, /\.studio-shell\.motion-enabled\.is-entering \.intro/);
	assert.match(css, /@media \(max-width: 360px\).*\.topline-label \{ display: none; \}/);
	assert.match(css, /@media \(prefers-reduced-motion: reduce\)/);
	assert.match(css, /animation-duration: \.01ms !important/);
});

test('disabling motion removes animated classes and persists the opt-out', () => {
	const studio = loadStudio({ motionEnabled: true });
	studio.handlers['root:click']({ target: { closest: () => ({ disabled: false, dataset: { action: 'motion' } }) } });
	assert.equal(studio.getPersisted().motionEnabled, false);
	assert.match(studio.root.innerHTML, /class="studio-shell"/);
	assert.doesNotMatch(studio.root.innerHTML, /is-entering|motion-enabled/);
});
