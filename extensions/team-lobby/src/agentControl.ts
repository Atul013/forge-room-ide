type SessionState = { paused?: boolean };
type ActiveRequest = { source: { cancel(): void }; session: SessionState; paused: boolean; discardOnCompletion?: boolean };

export function pauseAgentRequest(active: ActiveRequest | undefined): boolean {
	if (!active || active.paused || active.discardOnCompletion) { return false; }
	active.paused = true;
	active.session.paused = true;
	active.source.cancel();
	return true;
}

export function discardAgentRequest(active: ActiveRequest | undefined): boolean {
	if (!active || active.discardOnCompletion) { return false; }
	active.discardOnCompletion = true;
	active.session.paused = false;
	active.source.cancel();
	return true;
}
