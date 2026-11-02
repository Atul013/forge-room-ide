export function formatBuildRequestFromPlan(originalRequest: string, reviewedPlan: string) {
	return `Implement the reviewed plan below. Keep its scope and verification steps. Original request: ${originalRequest || '(see plan)'}\n\nReviewed implementation plan:\n${reviewedPlan}`;
}
