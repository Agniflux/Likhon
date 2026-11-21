/* C interface to the Likhon typing engine (Desktop/Source/crates/likhon-ffi/src/lib.rs). MPL-2.0.
 *
 * One session per text field. Strings come back in `commit` and `preedit` (UTF-8, possibly empty) and stay valid
 * until the next call on the same session. Return value: 1 = Likhon used the key, 0 = the host should handle it. */
#ifndef LIKHON_H
#define LIKHON_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct LikhonSession LikhonSession;

const char *likhon_version(void);
LikhonSession *likhon_session_new(void);
void likhon_session_free(LikhonSession *s);

/* Bangla digits, full stop as dari, colon as visarga (0 or 1 each). Ends the current word. */
void likhon_session_set_options(LikhonSession *s, int bangla_digits, int dari, int visarga);

/* Opens the dictionary (UTF-8 path) for suggestions and autocorrect. 1 = loaded, 0 = missing or not a Likhon
 * dictionary (typing still works, without suggestions). */
int likhon_session_load_dictionary(LikhonSession *s, const char *path);

/* Applies a settings file's text (UTF-8 JSON, the same names as Windows' settings.json: "layout", "typing.*").
 * 1 = applied, 0 = not JSON (nothing changes). Ends the word being typed. */
int likhon_session_apply_settings(LikhonSession *s, const char *json);
/* Opens the person's own words (user.sqlite, created if missing) for "my words" rows and learning. */
int likhon_session_open_user_data(LikhonSession *s, const char *path);
/* 1 for a password or PIN field: no suggestions, nothing learned. */
void likhon_session_set_private(LikhonSession *s, int private_field);

/* A typed character as a Unicode scalar value (already shifted). */
int likhon_session_key(LikhonSession *s, uint32_t ch, const char **commit, const char **preedit);
int likhon_session_backspace(LikhonSession *s, const char **commit, const char **preedit);
int likhon_session_cancel(LikhonSession *s, const char **commit, const char **preedit);
/* Up (down = 0) or Down (1) in the suggestion list. Returns 0 when there is no list. */
int likhon_session_move(LikhonSession *s, int down, const char **commit, const char **preedit);
/* Enter: types the selected suggestion and ends the word. 1 = it did (swallow the Enter). */
int likhon_session_accept(LikhonSession *s, const char **commit, const char **preedit);
/* A number key: suggestion row `index` (0-based) is typed, with a space after it. 0 = no such row. */
int likhon_session_pick(LikhonSession *s, int index, const char **commit, const char **preedit);

/* The suggestion rows after the last call: how many, row i (UTF-8, valid until the next call), the selected one. */
int likhon_session_row_count(const LikhonSession *s);
const char *likhon_session_row(const LikhonSession *s, int i);
int likhon_session_selected(const LikhonSession *s);

/* The word ends (focus out, a shortcut): `commit` holds the word as shown. */
int likhon_session_reset(LikhonSession *s, const char **commit, const char **preedit);

#ifdef __cplusplus
}
#endif

#endif
