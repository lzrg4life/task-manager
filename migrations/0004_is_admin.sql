ALTER TABLE users ADD COLUMN is_admin INTEGER NOT NULL DEFAULT 0;

UPDATE users
SET is_admin = 1
WHERE id IN (
    SELECT ur.user_id
    FROM user_roles ur
    JOIN roles r ON r.id = ur.role_id
    WHERE r.name = 'admin'
);

DROP TABLE user_roles;
DROP TABLE roles;
