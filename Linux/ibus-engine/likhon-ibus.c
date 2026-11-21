/* Likhon IBus engine (Linux). MPL-2.0.
 *
 * A thin shell around the Rust typing engine (likhon-ffi): it shows the word being typed as an underlined
 * preedit and commits it when the word ends. No file is read and nothing typed is stored or sent anywhere.
 *
 * Run by ibus-daemon as `ibus-engine-likhon --ibus`; `-v` prints the version. */

#include <glib/gstdio.h>
#include <ibus.h>
#include <stdlib.h>
#include <string.h>
#include "likhon.h"

#define ENGINE_NAME "likhon"
#define BUS_NAME "app.likhon.IBus"

typedef struct _IBusLikhonEngine IBusLikhonEngine;
typedef struct _IBusLikhonEngineClass IBusLikhonEngineClass;

struct _IBusLikhonEngine {
    IBusEngine parent;
    LikhonSession *session;
    IBusLookupTable *table;
    gint64 settings_mtime; /* of ~/.config/likhon/settings.json when last read; 0 = none */
};

struct _IBusLikhonEngineClass {
    IBusEngineClass parent;
};

#define IBUS_TYPE_LIKHON_ENGINE (ibus_likhon_engine_get_type())
GType ibus_likhon_engine_get_type(void);
G_DEFINE_TYPE(IBusLikhonEngine, ibus_likhon_engine, IBUS_TYPE_ENGINE)

#define DICTIONARY_PATH "/usr/share/ibus-likhon/words.sqlite"

/* ~/.config/likhon/settings.json (the same names as the Windows app's settings.json). Read when the engine starts
 * and again at each focus-in if the file changed, so an edit applies without restarting IBus. */
static void load_settings(IBusLikhonEngine *e) {
    gchar *path = g_build_filename(g_get_user_config_dir(), "likhon", "settings.json", NULL);
    GStatBuf st;
    gint64 mtime = 0;
    if (g_stat(path, &st) == 0)
        mtime = (gint64)st.st_mtime;
    if (mtime != e->settings_mtime) {
        e->settings_mtime = mtime;
        gchar *text = NULL;
        if (mtime != 0 && g_file_get_contents(path, &text, NULL, NULL)) {
            likhon_session_apply_settings(e->session, text);
            g_free(text);
        } else {
            likhon_session_apply_settings(e->session, "{}"); /* the file went away: back to the defaults */
        }
    }
    g_free(path);
}

/* The suggestion list: the rows the session offers, the selected one highlighted; hidden when there are none. */
static void show_rows(IBusLikhonEngine *e) {
    IBusEngine *engine = IBUS_ENGINE(e);
    int n = likhon_session_row_count(e->session);
    if (n <= 0) {
        ibus_engine_hide_lookup_table(engine);
        return;
    }
    ibus_lookup_table_clear(e->table);
    for (int i = 0; i < n; i++)
        ibus_lookup_table_append_candidate(e->table, ibus_text_new_from_string(likhon_session_row(e->session, i)));
    ibus_lookup_table_set_cursor_pos(e->table, (guint)likhon_session_selected(e->session));
    ibus_engine_update_lookup_table(engine, e->table, TRUE);
}

/* Shows `commit` (if any), the preedit (or hides it) and the suggestion list. */
static void show(IBusLikhonEngine *e, const char *commit, const char *preedit) {
    IBusEngine *engine = IBUS_ENGINE(e);
    show_rows(e);
    if (commit && *commit)
        ibus_engine_commit_text(engine, ibus_text_new_from_string(commit));
    if (preedit && *preedit) {
        IBusText *t = ibus_text_new_from_string(preedit);
        ibus_text_append_attribute(t, IBUS_ATTR_TYPE_UNDERLINE, IBUS_ATTR_UNDERLINE_SINGLE, 0, -1);
        ibus_engine_update_preedit_text(engine, t, g_utf8_strlen(preedit, -1), TRUE);
    } else {
        ibus_engine_hide_preedit_text(engine);
    }
}

/* The word ends: commit it as shown. The key itself is left for the application. */
static void end_word(IBusLikhonEngine *e) {
    const char *commit = NULL, *preedit = NULL;
    likhon_session_reset(e->session, &commit, &preedit);
    show(e, commit, preedit);
}

static gboolean process_key_event(IBusEngine *engine, guint keyval, guint keycode, guint state) {
    IBusLikhonEngine *e = (IBusLikhonEngine *)engine;
    const char *commit = NULL, *preedit = NULL;
    (void)keycode;

    if (state & IBUS_RELEASE_MASK)
        return FALSE;

    /* Shortcuts belong to the application: end the word and let the key through. */
    if (state & (IBUS_CONTROL_MASK | IBUS_MOD1_MASK | IBUS_MOD4_MASK | IBUS_SUPER_MASK)) {
        end_word(e);
        return FALSE;
    }

    if (keyval == IBUS_KEY_BackSpace) {
        int used = likhon_session_backspace(e->session, &commit, &preedit);
        if (used)
            show(e, commit, preedit);
        return used;
    }
    if (keyval == IBUS_KEY_Escape) {
        int used = likhon_session_cancel(e->session, &commit, &preedit);
        if (used)
            show(e, commit, preedit);
        return used;
    }

    /* The suggestion list is open: Up and Down move, Enter types the selected row. */
    if (likhon_session_row_count(e->session) > 0) {
        if (keyval == IBUS_KEY_Up || keyval == IBUS_KEY_Down) {
            int used = likhon_session_move(e->session, keyval == IBUS_KEY_Down, &commit, &preedit);
            show(e, commit, preedit);
            return used;
        }
        if (keyval == IBUS_KEY_Return || keyval == IBUS_KEY_KP_Enter) {
            int used = likhon_session_accept(e->session, &commit, &preedit);
            show(e, commit, preedit);
            return used;
        }
    }

    /* Shift and Caps Lock alone do not end a word. */
    if (keyval == IBUS_KEY_Shift_L || keyval == IBUS_KEY_Shift_R || keyval == IBUS_KEY_Caps_Lock)
        return FALSE;

    gunichar ch = ibus_keyval_to_unicode(keyval);
    if (ch == 0 || ch < 0x20 || ch == 0x7f || keyval == IBUS_KEY_Return || keyval == IBUS_KEY_KP_Enter) {
        /* Enter, Tab, arrows, function keys...: the word ends, the key goes on. */
        end_word(e);
        return FALSE;
    }

    int used = likhon_session_key(e->session, ch, &commit, &preedit);
    show(e, commit, preedit);
    return used;
}

static void focus_in(IBusEngine *engine) {
    load_settings((IBusLikhonEngine *)engine);
}

/* Password and PIN boxes: no suggestions, nothing learned. */
static void set_content_type(IBusEngine *engine, guint purpose, guint hints) {
    IBusLikhonEngine *e = (IBusLikhonEngine *)engine;
    (void)hints;
    likhon_session_set_private(e->session, purpose == IBUS_INPUT_PURPOSE_PASSWORD || purpose == IBUS_INPUT_PURPOSE_PIN);
    ibus_engine_hide_lookup_table(engine);
}

static void focus_out(IBusEngine *engine) {
    end_word((IBusLikhonEngine *)engine);
}

static void reset(IBusEngine *engine) {
    IBusLikhonEngine *e = (IBusLikhonEngine *)engine;
    /* The application moved the caret or changed the text: whatever was being typed is not ours to commit. */
    const char *commit = NULL, *preedit = NULL;
    likhon_session_cancel(e->session, &commit, &preedit);
    ibus_engine_hide_preedit_text(engine);
}

static void disable(IBusEngine *engine) {
    end_word((IBusLikhonEngine *)engine);
}

static void ibus_likhon_engine_init(IBusLikhonEngine *e) {
    e->session = likhon_session_new();
    /* LIKHON_DICTIONARY lets a development build point at another file. Typing works without a dictionary. */
    const char *path = g_getenv("LIKHON_DICTIONARY");
    likhon_session_load_dictionary(e->session, path && *path ? path : DICTIONARY_PATH);
    /* My words and learning: ~/.local/share/likhon/user.sqlite, readable by this user only. */
    gchar *dir = g_build_filename(g_get_user_data_dir(), "likhon", NULL);
    if (g_mkdir_with_parents(dir, 0700) == 0) {
        gchar *db = g_build_filename(dir, "user.sqlite", NULL);
        likhon_session_open_user_data(e->session, db);
        g_free(db);
    }
    g_free(dir);
    load_settings(e);
    e->table = ibus_lookup_table_new(5, 0, FALSE, TRUE);
    g_object_ref_sink(e->table);
}

static void ibus_likhon_engine_finalize(GObject *object) {
    IBusLikhonEngine *e = (IBusLikhonEngine *)object;
    likhon_session_free(e->session);
    e->session = NULL;
    g_clear_object(&e->table);
    G_OBJECT_CLASS(ibus_likhon_engine_parent_class)->finalize(object);
}

static void ibus_likhon_engine_class_init(IBusLikhonEngineClass *klass) {
    GObjectClass *object_class = G_OBJECT_CLASS(klass);
    IBusEngineClass *engine_class = IBUS_ENGINE_CLASS(klass);
    object_class->finalize = ibus_likhon_engine_finalize;
    engine_class->process_key_event = process_key_event;
    engine_class->focus_in = focus_in;
    engine_class->focus_out = focus_out;
    engine_class->reset = reset;
    engine_class->set_content_type = set_content_type;
    engine_class->disable = disable;
}

static void on_disconnected(IBusBus *bus, gpointer user_data) {
    (void)bus;
    (void)user_data;
    ibus_quit();
}

int main(int argc, char **argv) {
    gboolean by_daemon = FALSE;
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "--ibus") == 0 || strcmp(argv[i], "-i") == 0)
            by_daemon = TRUE;
        else if (strcmp(argv[i], "-v") == 0 || strcmp(argv[i], "--version") == 0) {
            g_print("ibus-engine-likhon %s\n", likhon_version());
            return 0;
        }
    }

    ibus_init();
    IBusBus *bus = ibus_bus_new();
    g_signal_connect(bus, "disconnected", G_CALLBACK(on_disconnected), NULL);

    IBusFactory *factory = ibus_factory_new(ibus_bus_get_connection(bus));
    ibus_factory_add_engine(factory, ENGINE_NAME, IBUS_TYPE_LIKHON_ENGINE);

    if (by_daemon) {
        ibus_bus_request_name(bus, BUS_NAME, 0);
    } else {
        /* Started by hand (development): register the component so the engine can be selected. */
        IBusComponent *component = ibus_component_new(BUS_NAME, "Likhon Bangla", likhon_version(), "MPL-2.0",
                                                      "Agniflux Enterprise", "https://likhon.agniflux.com", "", "ibus-likhon");
        ibus_component_add_engine(component, ibus_engine_desc_new(ENGINE_NAME, "Likhon Bangla (Phonetic)", "Bangla phonetic typing",
                                                                  "bn", "MPL-2.0", "Agniflux Enterprise", "", "us"));
        ibus_bus_register_component(bus, component);
    }

    ibus_main();
    return 0;
}
