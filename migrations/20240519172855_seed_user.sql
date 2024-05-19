-- Add migration script here
INSERT INTO users (user_id, username, password_hash)
VALUES (
  '1a54867d-288f-4167-9108-70af0295efd6',
  'admin',
  '$argon2id$v=19$m=15000,t=2,p=1$OEx/rcq+3ts//'
  'WUDzGNl2g$Am8UFBA4w5NJEmAtquGvBmAlu92q/VQcaoL5AyJPfc8'
)
