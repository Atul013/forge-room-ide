(function () {
	const vscode = window.vscode;
	const app = document.getElementById('app');
	let snapshot = window.roomSnapshot;
	let hasRendered = false;
	const send = (command, extra) => vscode.postMessage(Object.assign({ command }, extra || {}));
	const esc = value => String(value == null ? '' : value).replace(/[&<>"']/g, char => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[char]);
	const initials = name => String(name || '?').split(/[\s._-]+/).filter(Boolean).slice(0, 2).map(part => part[0]).join('').toUpperCase();
	const time = value => { const date = new Date(value); return Number.isNaN(date.getTime()) ? '' : date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }); };
	const relative = value => { const elapsed = Math.max(0, Math.floor((Date.now() - new Date(value).getTime()) / 60000)); return elapsed < 1 ? 'just now' : elapsed < 60 ? `${elapsed}m ago` : `${Math.floor(elapsed / 60)}h ago`; };
	const statusLabel = value => value === 'inProgress' ? 'In motion' : value === 'done' ? 'Done' : 'Open';
	const countdownLabel = value => {
		const timestamp = Date.parse(value || '');
		if (!Number.isFinite(timestamp)) { return ''; }
		const remaining = timestamp - Date.now();
		if (remaining <= 0) { return 'Submission deadline reached'; }
		const minutes = Math.ceil(remaining / 60000);
		const days = Math.floor(minutes / 1440);
		const hours = Math.floor(minutes % 1440 / 60);
		const mins = minutes % 60;
		return `${days ? `${days}d ` : ''}${hours}h ${mins}m remaining`;
	};
	const taskDeadline = value => {
		const timestamp = Date.parse(value || '');
		if (!Number.isFinite(timestamp)) { return ''; }
		const delta = timestamp - Date.now();
		const minutes = Math.ceil(Math.abs(delta) / 60000);
		const duration = minutes < 60 ? `${minutes}m` : minutes < 1440 ? `${Math.floor(minutes / 60)}h ${minutes % 60}m` : `${Math.floor(minutes / 1440)}d ${Math.floor(minutes % 1440 / 60)}h`;
		const label = delta < 0 ? `Overdue ${duration}` : `Due in ${duration}`;
		return `<span class="task-deadline" data-overdue="${delta < 0}" title="${esc(new Date(timestamp).toLocaleString())}">${esc(label)}</span>`;
	};
	const commentLocation = event => event.anchor ? (event.anchor.startLine === event.anchor.endLine ? `line ${event.anchor.startLine}` : `lines ${event.anchor.startLine}–${event.anchor.endLine}`) : '';

	function render() {
		const enterClass = hasRendered ? '' : ' is-entering';
		const members = snapshot.members || [];
		const tasks = snapshot.tasks || [];
		const memberTasks = member => {
			const participantId = member.ownerId || (member.isSelf === true || member.id === 'self' ? snapshot.participantId : member.id);
			const sameNameCount = members.filter(item => item.name === member.name).length;
			return tasks.filter(task => task.status !== 'done' && (task.ownerId ? task.ownerId === participantId : sameNameCount === 1 && task.owner === member.name));
		};
		const notes = snapshot.notes || [];
		const viewerId = snapshot.participantId;
		const events = [...(snapshot.events || [])].filter(event => event.kind !== 'message' || !event.recipient || !!viewerId && (event.recipientId === viewerId || event.senderId === viewerId)).reverse().slice(0, 8);
		const online = snapshot.connected || !!snapshot.hosting;
		const project = snapshot.project || 'Open a project to start a room';
		const countdownPaused = Number.isSafeInteger(snapshot.sessionPausedRemainingMs) && snapshot.sessionPausedRemainingMs >= 0;
		const countdownText = countdownPaused ? countdownLabel(new Date(Date.now() + snapshot.sessionPausedRemainingMs).toISOString()) : countdownLabel(snapshot.sessionDeadline);
		const countdownSection = countdownText ? `<section class="hackathon-countdown" role="timer" aria-label="Hackathon countdown"><span>${countdownPaused ? 'Countdown paused' : 'Submission deadline'}</span><strong>${countdownPaused ? `Paused · ${esc(countdownText)}` : esc(countdownText)}</strong><small>${countdownPaused ? 'Time is frozen until the host resumes the countdown.' : esc(new Date(snapshot.sessionDeadline).toLocaleString())}</small></section>` : '';
		const liveDiffSection = snapshot.liveDiffSharing ? `<section class="section" aria-label="Live Git diff sharing"><div class="section-head"><h2 class="section-title">Live Git diff sharing</h2><button class="button button-quiet" data-action="stopLiveDiff">Stop sharing</button></div><div class="event-meta">New saved diffs for ${esc(snapshot.liveDiffFile || 'the selected file')} are shared with the room. Earlier snapshots remain in room history.</div></section>` : '';
		const openTasks = tasks.filter(task => task.status !== 'done').length;
		const visibleTasks = tasks.slice().sort((a, b) => ({ inProgress: 0, open: 1, done: 2 }[a.status] - { inProgress: 0, open: 1, done: 2 }[b.status]) || (a.dueAt ? Date.parse(a.dueAt) : Infinity) - (b.dueAt ? Date.parse(b.dueAt) : Infinity)).slice(0, 6);
		const noteSection = `<section class="section"><div class="section-head"><h2 class="section-title">Room notes</h2><span class="section-meta">${notes.length} saved</span></div>${notes.length ? `<div class="note-list">${notes.map(note => `<button class="note-card" data-action="openNote" data-id="${esc(note.id)}"><span class="note-title">${esc(note.title)}</span><span class="note-body">${esc(note.body)}</span><span class="note-meta">${esc(note.owner)} · ${relative(note.updatedAt)}</span></button>`).join('')}</div>` : `<div class="empty"><strong>No shared notes yet.</strong>Keep decisions and handoffs visible to everyone in the room.${online ? '<button class="empty-action" data-action="createNote">Add a room note</button>' : ''}</div>`}${notes.length && online ? '<button class="empty-action" data-action="createNote">Add a room note</button>' : ''}</section>`;
		app.innerHTML = `<main class="room${enterClass}">
			<header class="masthead"><div class="brand"><span class="brand-mark">F</span><span>Forge Room</span></div><div class="connection" role="status" aria-live="polite" data-live="${online}"><span class="signal" aria-hidden="true"></span><span>${online ? (snapshot.hosting ? 'Hosting' : 'Connected') : 'Offline'}</span></div></header>
			<section class="hero"><div class="eyebrow">Project room</div><div class="hero-row"><div><h1>${esc(project)}</h1><div class="project">${online ? 'Work is happening here.' : 'A shared space for the work ahead.'}</div></div><div class="count"><strong>${members.length.toString().padStart(2, '0')}</strong>here now</div></div></section>
			<div class="actions">${online ? '<button class="button button-primary" data-action="post">Share an update <span class="arrow">↗</span></button><button class="button" data-action="message">Message team</button><button class="button button-quiet" data-action="task">New task</button><button class="button button-quiet" data-action="shareDiff">Share a Git diff</button>' : '<button class="button button-primary" data-action="host">Host a room <span class="arrow">↗</span></button><button class="button" data-action="join">Join a room</button>'}</div>
			<section class="section"><div class="section-head"><h2 class="section-title">In the room</h2><span class="section-meta">${members.length ? `${members.length} present` : 'awaiting team'}</span></div>${members.length ? `<div class="people">${members.map(member => { const owned = memberTasks(member); const summary = owned.length ? owned.slice(0, 2).map(task => `${task.title} · ${statusLabel(task.status)}`).join(' / ') + (owned.length > 2 ? ` / +${owned.length - 2} more` : '') : 'No active task'; const presence = member.role === 'agent' && member.status ? ` · ${member.status}${member.ownerId ? ` · owned by ${members.find(owner => owner.id === member.ownerId)?.name || 'teammate'}` : ''}` : ''; return `<div class="person"><span class="avatar ${member.role === 'agent' ? 'agent' : ''}" aria-hidden="true">${esc(initials(member.name))}</span><div class="person-copy"><div class="person-name">${esc(member.name)}${member.isSelf === true || member.id === 'self' ? ' <span class="person-role">you</span>' : ''}</div><div class="person-role">${esc(member.role + presence)}</div><div class="person-task" title="${esc(summary)}">${owned.length ? `Working on ${esc(summary)}` : esc(summary)}</div></div><span class="person-state"></span></div>`; }).join('')}</div>` : `<div class="empty"><strong>Your room starts with people.</strong>Host a room and invite the team, or join with an invite from a teammate.</div>`}</section>
			<section class="section"><div class="section-head"><h2 class="section-title">Current work</h2><span class="section-meta">${openTasks} open · ${tasks.filter(task => task.status !== 'done' && (task.dependencies || []).some(id => tasks.some(item => item.id === id && item.status !== 'done'))).length} blocked</span></div>${tasks.length ? `<div class="task-list">${visibleTasks.map(task => { const blockers = (task.dependencies || []).map(id => tasks.find(item => item.id === id)).filter(item => item && item.status !== 'done'); const deadline = task.status === 'done' ? '' : taskDeadline(task.dueAt); const deadlineText = deadline.replace(/<[^>]*>/g, ''); return `<div class="task" role="button" tabindex="0" aria-label="Open task: ${esc(task.title)}${deadlineText ? `. ${esc(deadlineText)}` : ''}${task.acceptanceCriteria?.length ? `. Acceptance criteria: ${esc(task.acceptanceCriteria.join('; '))}` : ''}" data-action="openEvent" data-id="${esc(task.id)}"><div class="task-top"><span class="task-title">${esc(task.title)}</span><span class="task-status" data-state="${esc(blockers.length ? 'blocked' : task.status)}">${blockers.length ? 'Blocked' : statusLabel(task.status)}</span></div><div class="task-owner">Assigned to ${esc(task.owner)}${task.worktreeBranch ? ` · ${esc(task.worktreeBranch)}` : ''} · ${relative(task.updatedAt)}${deadline ? ` · ${deadline}` : ''}${blockers.length ? ` · waiting on ${esc(blockers.map(item => item.title).join(', '))}` : ''}</div>${task.acceptanceCriteria?.length ? `<div class="task-criteria">${task.acceptanceCriteria.map(criterion => { const confirmed = task.completedCriteria?.includes(criterion) === true; return `<div class="task-criterion" data-confirmed="${confirmed}"><span aria-hidden="true">${confirmed ? '✓' : '○'}</span> ${esc(criterion)}${confirmed ? ' · owner confirmed' : ''}</div>`; }).join('')}</div>` : ''}</div>`; }).join('')}</div>` : `<div class="empty"><strong>No tasks on the board.</strong>Give the next piece of work an owner and a clear finish line.${online ? '<button class="empty-action" data-action="task">Add the first task</button>' : ''}</div>`}</section>
			<section class="section"><div class="section-head"><h2 class="section-title">Room pulse</h2><span class="section-meta">${events.length ? 'latest first' : 'live activity'}</span></div>${events.length ? `<div class="feed">${events.map(event => `<div class="event" ${event.kind === 'diff' || event.kind === 'revoke' || event.kind === 'agent' ? '' : 'role="button" tabindex="0" data-action="openEvent"'} ${event.kind === 'diff' || event.kind === 'revoke' || event.kind === 'agent' ? '' : `aria-label="Open activity: ${esc(event.name)} ${esc(event.detail)}"`} data-kind="${esc(event.kind)}" data-id="${esc(event.id)}"><span class="event-mark"></span><div><div class="event-detail">${esc(event.name)} ${({ save: 'saved', edit: 'is editing', join: 'joined the room', leave: 'left the room', task: 'updated a task', agent: 'worked with Forge Room Agent', message: 'sent a message', post: event.line !== undefined ? 'shared a location in' : 'shared an update', commit: 'committed', merge: 'merged', test: 'ran tests', review: 'shared a review', handoff: 'handed work off', note: 'updated a room note', diff: 'shared a Git diff for', comment: 'commented on', revoke: 'changed room access', conflict: event.detail.startsWith('Git reports unresolved merge stages') ? 'reported a Git merge conflict in' : 'flagged a possible edit overlap on' })[event.kind] || 'shared an update'}${event.file ? ` <span>${esc(event.file)}${event.kind === 'comment' ? `:${commentLocation(event) ? ` ${esc(commentLocation(event))}` : ''}` : event.line !== undefined ? `:${event.line}` : ''}</span>${event.kind === 'conflict' || event.kind === 'comment' || event.kind === 'revoke' ? `: ${esc(event.detail)}` : ''}` : ['message', 'agent', 'post', 'commit', 'merge', 'test', 'review', 'handoff', 'note', 'conflict', 'revoke'].includes(event.kind) ? `: ${esc(event.detail)}` : ''}</div><div class="event-meta">${esc(event.role)} · ${relative(event.at)}${event.recipient ? ` · to ${esc(event.recipient)}` : ''}</div>${event.kind === 'diff' ? `<button class="diff-open" aria-label="Review shared diff for ${esc(event.file || event.detail)}" data-action="viewDiff" data-id="${esc(event.id)}">Review shared diff</button>` : event.line !== undefined ? `<div class="event-meta">Open shared file location (path and line only)</div>` : event.kind === 'comment' ? `<div class="event-meta">Open the saved line anchor · selected code was not shared</div>` : ''}</div></div>`).join('')}</div>` : `<div class="empty"><strong>The room is quiet for now.</strong>${online ? 'Share a note or save a file to start the activity feed.' : 'When your team joins, their work will appear here.'}</div>`}</section>
			${noteSection}
			${countdownSection}
			${liveDiffSection}
			${snapshot.hosting ? `<div class="actions countdown-actions"><button class="button button-quiet" data-action="setCountdown">${snapshot.sessionDeadline ? 'Change or clear deadline' : countdownPaused ? 'Change deadline' : 'Set submission deadline'}</button>${snapshot.sessionDeadline || countdownPaused ? `<button class="button button-quiet" data-action="toggleCountdown">${countdownPaused ? 'Resume countdown' : 'Pause countdown'}</button>` : ''}</div>` : ''}
			<footer class="footer"><span>${tasks.length} ${tasks.length === 1 ? 'task' : 'tasks'} · ${notes.length} notes</span>${online ? '<button data-action="leave">Leave room</button>' : '<span>Local collaboration</span>'}</footer>
		</main>`;
		hasRendered = true;
	}

	window.addEventListener('message', event => { if (event.data && event.data.type === 'snapshot') { snapshot = event.data.snapshot; render(); } });
	setInterval(() => { if (snapshot?.sessionDeadline || snapshot?.tasks?.some(task => task.dueAt)) { render(); } }, 60_000);
	app.addEventListener('keydown', event => {
		if ((event.key !== 'Enter' && event.key !== ' ') || event.target.closest('button')) { return; }
		const target = event.target.closest('[role=\"button\"][data-action]');
		if (!target || target.matches('.diff-open')) { return; }
		event.preventDefault();
		if (target.dataset.action === 'openEvent' && !target.dataset.id) { return; }
		send(target.dataset.action, { id: target.dataset.id, file: target.dataset.file });
	});
	app.addEventListener('click', event => {
		const target = event.target.closest('[data-action]');
		if (!target) { return; }
		const command = target.dataset.action;
		if (command === 'openEvent' && !target.dataset.id) { return; }
		send(command, { id: target.dataset.id, file: target.dataset.file });
	});
	render();
}());
