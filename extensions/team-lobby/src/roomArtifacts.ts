import { validCommentAnchor, type CommentAnchor } from './commentAnchor';

export type RoomArtifact = {
	kind: 'diff' | 'comment';
	detail: string;
	file: string;
	patch?: string;
	anchor?: CommentAnchor;
};

const maxArtifactDetail = 500;
const maxArtifactPatch = 8_000;
const privateArtifactFields = ['content', 'prompt', 'selectedText', 'sourceText', 'terminalOutput'];

/** Validate only the data needed for a reviewed room diff or code-anchored comment. */
export function validRoomArtifact(value: unknown): value is RoomArtifact {
	if (!value || typeof value !== 'object' || Array.isArray(value)) { return false; }
	const artifact = value as Record<string, unknown>;
	if (privateArtifactFields.some(key => Object.hasOwn(artifact, key))) { return false; }
	if ((artifact.kind !== 'comment' && artifact.kind !== 'diff') || typeof artifact.detail !== 'string' || !artifact.detail.length || artifact.detail.length > maxArtifactDetail || typeof artifact.file !== 'string' || !artifact.file.length || artifact.file.length > 500 || /[\u0000-\u001f\u007f]/.test(artifact.file) || artifact.file.startsWith('/') || /^[a-zA-Z]:/.test(artifact.file) || artifact.file.split(/[\\/]/).includes('..')) { return false; }
	if (artifact.kind === 'comment') { return artifact.patch === undefined && validCommentAnchor(artifact.anchor); }
	return artifact.anchor === undefined && typeof artifact.patch === 'string' && artifact.patch.length > 0 && artifact.patch.length <= maxArtifactPatch;
}

