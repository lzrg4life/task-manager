ALTER TABLE tasks
ADD COLUMN editing_user_id INTEGER REFERENCES users(id);
