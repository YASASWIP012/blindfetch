// Every film in the project, by id → compositions <id>-16x9, <id>-9x16, <id>-1x1, <id>-4x5.
// Add a film: put <id>.json in this folder, import it below and add it to FILMS.
import starter30 from './starter-30.json';
import starterLong from './starter-long.json';
// @imports
import film_blindfetch from './blindfetch.json';
import film_gloss_long from './gloss-long.json';
import film_gloss_30 from './gloss-30.json';

export const FILMS: Record<string, unknown> = {
  'starter-30': starter30,
  'starter-long': starterLong,
  // @films
  'blindfetch': film_blindfetch,
  'gloss-long': film_gloss_long,
  'gloss-30': film_gloss_30,
};
