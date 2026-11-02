export const maxDecisionTitleLength = 140;
export const maxDecisionRationaleLength = 340;

export function formatRoomDecision(title: string, rationale: string) {
	const cleanTitle = title.trim();
	const cleanRationale = rationale.trim();
	if (!cleanTitle || cleanTitle.length > maxDecisionTitleLength || /[\r\n\0]/.test(cleanTitle) || !cleanRationale || cleanRationale.length > maxDecisionRationaleLength || /[\r\n\0]/.test(cleanRationale)) { return undefined; }
	return `Decision: ${cleanTitle}\nRationale: ${cleanRationale}`;
}

export function isValidRoomDecisionDetail(detail: unknown): detail is string {
	if (typeof detail !== 'string' || detail.length > 500) { return false; }
	const match = /^Decision: ([^\r\n]{1,140})\nRationale: ([^\r\n]{1,340})$/.exec(detail);
	return !!match && formatRoomDecision(match[1], match[2]) === detail;
}
