// Shared helpers for the e2e suite: everything talks to the docker
// stack, so database seeding goes through psql in the postgres
// container.
import { execSync } from 'node:child_process';
import { randomBytes } from 'node:crypto';

export function psql(sql: string): string {
	// Collapse the statement to one line: JSON.stringify would smuggle
	// literal \n escapes into the shell string.
	const flat = sql.replace(/\s+/g, ' ').trim();
	return execSync(
		`docker exec mutamarket-postgres psql -U mutamarket -d mutamarket -tAc ${JSON.stringify(flat)}`,
		{ encoding: 'utf8' },
	).trim();
}

/** A fresh session token for the first admin user in the database. */
export function adminSessionToken(): string {
	const userId = psql('select id from users where is_admin order by id limit 1');
	if (!userId) {
		throw new Error('no admin user in the database - run the legacy import or flip is_admin');
	}
	const token = randomBytes(24).toString('hex');
	psql(
		`insert into sessions (token, user_id, expires_at)
		 values ('${token}', ${userId}, now() + interval '1 hour')`,
	);
	return token;
}
