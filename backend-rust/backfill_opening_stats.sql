-- Backfill precomputed opening stats tables
-- Run once after deploying the schema changes

-- 1. Backfill game_opening_mistakes
INSERT INTO game_opening_mistakes (game_id, user_id, ply, move_san, cp_loss, best_move, color, line)
SELECT
  ug.id,
  ug.user_id,
  (ord - 1)::smallint,
  (m->>'move')::text,
  (m->>'cp_loss')::float8,
  (m->>'best_move')::text,
  LOWER(ug.user_color),
  (SELECT string_agg(elem->>'move', ' ' ORDER BY o)
   FROM jsonb_array_elements(ga.moves) WITH ORDINALITY AS y(elem, o)
   WHERE o <= x.ord)
FROM user_games ug
INNER JOIN game_analysis ga ON ug.id = ga.game_id,
LATERAL jsonb_array_elements(ga.moves) WITH ORDINALITY AS x(m, ord)
WHERE (ord - 1) < 30
  AND (m->>'cp_loss')::float8 >= 50
  AND COALESCE((m->>'classification')::text, '') NOT IN ('book', 'forced')
  AND (
    (LOWER(ug.user_color) = 'white' AND (ord - 1) % 2 = 0)
    OR (LOWER(ug.user_color) = 'black' AND (ord - 1) % 2 = 1)
  )
ON CONFLICT DO NOTHING;

-- 2. Backfill game_opening_clean_plies
WITH game_first_mistake AS (
  SELECT
    ug.id AS game_id,
    ug.user_id,
    LOWER(ug.user_color) AS color,
    MIN(CASE
      WHEN (m->>'cp_loss')::float8 >= 50
           AND COALESCE((m->>'classification')::text, '') NOT IN ('book', 'forced')
           AND (
             (LOWER(ug.user_color) = 'white' AND (ord - 1) % 2 = 0)
             OR (LOWER(ug.user_color) = 'black' AND (ord - 1) % 2 = 1)
           )
      THEN (ord - 1)::int
      ELSE NULL
    END) AS first_mistake_ply
  FROM user_games ug
  INNER JOIN game_analysis ga ON ug.id = ga.game_id,
  LATERAL jsonb_array_elements(ga.moves) WITH ORDINALITY AS x(m, ord)
  WHERE (ord - 1) < 30
  GROUP BY ug.id, ug.user_id, ug.user_color
),
clean_lines AS (
  SELECT
    gfm.game_id,
    gfm.user_id,
    gfm.color,
    LEAST(
      CASE
        WHEN gfm.first_mistake_ply IS NOT NULL THEN gfm.first_mistake_ply - 1
        WHEN gfm.color = 'white' THEN 29
        ELSE 30
      END,
      jsonb_array_length(ga.moves)
    ) AS clean_up_to,
    (SELECT string_agg(elem->>'move', ' ' ORDER BY o)
     FROM jsonb_array_elements(ga.moves) WITH ORDINALITY AS y(elem, o)
     WHERE (o - 1) < LEAST(
       CASE
         WHEN gfm.first_mistake_ply IS NOT NULL THEN gfm.first_mistake_ply - 1
         WHEN gfm.color = 'white' THEN 29
         ELSE 30
       END,
       jsonb_array_length(ga.moves)
     )) AS line,
    (SELECT COALESCE(AVG((elem->>'cp_loss')::float8), 0)
     FROM jsonb_array_elements(ga.moves) WITH ORDINALITY AS y(elem, o)
     WHERE (o - 1) < LEAST(
       CASE
         WHEN gfm.first_mistake_ply IS NOT NULL THEN gfm.first_mistake_ply - 1
         WHEN gfm.color = 'white' THEN 29
         ELSE 30
       END,
       jsonb_array_length(ga.moves)
     )
     AND (
       (gfm.color = 'white' AND (o - 1) % 2 = 0)
       OR (gfm.color = 'black' AND (o - 1) % 2 = 1)
     )) AS avg_cp_loss
  FROM game_first_mistake gfm
  INNER JOIN game_analysis ga ON gfm.game_id = ga.game_id
)
INSERT INTO game_opening_clean_plies (game_id, user_id, color, clean_up_to, clean_depth, line, avg_cp_loss)
SELECT
  game_id, user_id, color,
  clean_up_to::smallint,
  (clean_up_to / 2 + clean_up_to % 2)::smallint,
  line,
  ROUND(avg_cp_loss::numeric, 1)::float8
FROM clean_lines
WHERE line IS NOT NULL
ON CONFLICT (game_id) DO UPDATE SET
  clean_up_to = EXCLUDED.clean_up_to,
  clean_depth = EXCLUDED.clean_depth,
  line = EXCLUDED.line,
  avg_cp_loss = EXCLUDED.avg_cp_loss;
