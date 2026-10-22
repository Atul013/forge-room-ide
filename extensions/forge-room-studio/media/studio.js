/*---------------------------------------------------------------------------------------------
 *  Copyright (c) Forge Room. All rights reserved.
 *  Licensed under the MIT License. See LICENSE in the project root for license information.
 *--------------------------------------------------------------------------------------------*/

(() => {
	const vscode = window.forgeStudio;
	const root = document.getElementById('studio');
	let state = { themes: window.studioInitial.themes, current: '', previous: false };
	let motionEnabled = vscode.getState()?.motionEnabled === true;
	let entranceAnimationPending = motionEnabled;
	let hasRendered = false;
	let hasReceivedState = false;
	const escapeHtml = value => String(value ?? '').replace(/[&<>"']/g, char => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[char]);
	const activeId = () => state.themes.find(theme => theme.id === state.current)?.id;
	const art = name => ({
		ronin: '<span class="sun"></span><span class="ridge ridge-a"></span><span class="ridge ridge-b"></span><span class="torii"><i></i><i></i></span>',
		orbit: '<span class="planet"></span><span class="orbit orbit-a"></span><span class="orbit orbit-b"></span><span class="star star-a"></span><span class="star star-b"></span>',
		moss: '<span class="moon"></span><span class="mountain mountain-a"></span><span class="mountain mountain-b"></span><span class="sprig sprig-a"></span><span class="sprig sprig-b"></span>',
		ember: '<span class="ember ember-a"></span><span class="ember ember-b"></span><span class="kiln"></span><span class="kiln-door"></span>',
		paper: '<span class="paper-sun"></span><span class="paper-hill paper-hill-a"></span><span class="paper-hill paper-hill-b"></span>',
		signal: '<span class="signal-bar signal-a"></span><span class="signal-bar signal-b"></span><span class="signal-bar signal-c"></span><span class="signal-target"></span>'
	})[name] || '';

	function preview(theme) {
		return `<div class="preview ${escapeHtml(theme.art)}"><div class="preview-toolbar"><span></span><span></span><span></span><b>${escapeHtml(theme.name)}</b></div><div class="preview-body"><div class="preview-rail"><i></i><i></i><i></i><i></i></div><div class="preview-code"><div><i>const</i> <b>room</b> = {</div><div class="indent"><i>signal:</i> <em>'ready'</em>,</div><div class="indent"><i>members:</i> <strong>4</strong></div><div>}</div><div class="preview-mark" aria-hidden="true">${art(theme.art)}</div></div></div></div>`;
	}

	function render() {
		const current = activeId();
		const enterClass = entranceAnimationPending ? ' is-entering' : '';
		const motionClass = motionEnabled ? ' motion-enabled' : '';
		entranceAnimationPending = false;
		root.innerHTML = `<main class="studio-shell${motionClass}${enterClass}">
			<header class="topline"><span class="brand-mark" aria-hidden="true">F</span><span class="brand-name">FORGE ROOM</span><span class="topline-rule"></span><span class="topline-label">STUDIO / 01</span><button class="motion-toggle" type="button" data-action="motion" aria-pressed="${motionEnabled}" title="Decorative motion is off by default and always follows the system reduced-motion setting">Motion ${motionEnabled ? 'on' : 'off'}</button></header>
			<section class="intro"><div class="intro-kicker">A workspace, in your key</div><h1>Change the<br><span>whole atmosphere.</span></h1><p>Six carefully tuned color worlds for the way you like to work.</p><div class="intro-art" aria-hidden="true"><span class="art-disc"></span><span class="art-line art-line-a"></span><span class="art-line art-line-b"></span><span class="art-shape"></span><span class="art-label">MAKE<br>SPACE</span></div></section>
			<div class="gallery-heading"><div><span class="micro-label">THE PALETTE</span><strong>${state.themes.length} WORKSPACE THEMES</strong></div><button class="restore" data-action="restore" ${state.previous ? '' : 'disabled'}>${state.previous ? 'Restore previous' : 'Previous saved on apply'} <span aria-hidden="true">↶</span></button></div>
			<section class="theme-list" aria-label="Workspace themes">${state.themes.map(theme => `<article class="theme theme-${escapeHtml(theme.key)}"><div class="theme-copy"><div><div class="theme-name">${escapeHtml(theme.name)}${current === theme.id ? '<span class="active-mark" aria-label="Current theme">CURRENT</span>' : ''}</div><div class="theme-mood">${escapeHtml(theme.mood)}</div></div><div class="swatches" aria-label="Theme color palette">${theme.colors.map((_, index) => `<span class="swatch-${index}"></span>`).join('')}</div></div>${preview(theme)}<div class="theme-bottom"><p>${escapeHtml(theme.description)}</p><button class="apply" data-action="apply" data-id="${escapeHtml(theme.id)}" ${current === theme.id ? 'disabled' : ''}>${current === theme.id ? 'Applied' : 'Apply theme'}<span aria-hidden="true">${current === theme.id ? '✓' : '↗'}</span></button></div></article>`).join('')}</section>
			<p class="sr-only" role="status" aria-live="polite">${hasReceivedState ? `Current workspace theme: ${escapeHtml(state.themes.find(theme => theme.id === current)?.name || 'unknown')}` : ''}</p><footer class="studio-footer"><span class="footer-symbol" aria-hidden="true">◌</span><p>Only appearance changes. Your files, extensions and keybindings stay as they are.</p><span class="footer-version">FR / 01</span></footer>
		</main>`;
		hasRendered = true;
	}

	root.addEventListener('click', event => {
		const button = event.target.closest('[data-action]');
		if (!button || button.disabled) { return; }
		if (button.dataset.action === 'motion') {
			motionEnabled = !motionEnabled;
			entranceAnimationPending = motionEnabled;
			vscode.setState({ motionEnabled });
			render();
			return;
		}
		vscode.postMessage(button.dataset.action === 'apply' ? { command: 'apply', id: button.dataset.id } : { command: 'restore' });
	});
	window.addEventListener('message', event => {
		if (event.data?.type === 'studio-state') { state = event.data; hasReceivedState = true; render(); }
	});
	render();
	vscode.postMessage({ command: 'ready' });
})();
