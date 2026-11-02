export type CommentAnchor = { startLine: number; endLine: number };

export function validCommentAnchor(value: unknown): value is CommentAnchor {
	if (!value || typeof value !== 'object') { return false; }
	const anchor = value as Record<string, unknown>;
	return Number.isInteger(anchor.startLine) && Number.isInteger(anchor.endLine)
		&& (anchor.startLine as number) >= 1
		&& (anchor.endLine as number) <= 10_000_000
		&& (anchor.endLine as number) >= (anchor.startLine as number)
		&& (anchor.endLine as number) - (anchor.startLine as number) < 200;
}

export function commentAnchorLabel(anchor: CommentAnchor) {
	return anchor.startLine === anchor.endLine ? `line ${anchor.startLine}` : `lines ${anchor.startLine}–${anchor.endLine}`;
}
