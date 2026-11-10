"""Explicit local corrections only with disable and clear controls"""
import sqlite3
from .ranking import relative_path
from .scanner import CATEGORIES


class Preferences:
    """Use an app owned database connection rather than a document supplied path"""
    def __init__(self, connection):
        if not isinstance(connection, sqlite3.Connection):
            raise ValueError('Provide an app owned SQLite connection')
        self.db = connection
        self.db.executescript('''
            CREATE TABLE IF NOT EXISTS preferences_settings
              (id INTEGER PRIMARY KEY CHECK(id=1), enabled INTEGER NOT NULL CHECK(enabled IN (0,1)));
            INSERT OR IGNORE INTO preferences_settings VALUES(1,1);
            CREATE TABLE IF NOT EXISTS corrections
              (scope TEXT NOT NULL, category TEXT NOT NULL, destination TEXT NOT NULL,
               PRIMARY KEY(scope,category));
        ''')

    @property
    def enabled(self):
        return bool(self.db.execute('SELECT enabled FROM preferences_settings WHERE id=1').fetchone()[0])

    def set_enabled(self, enabled):
        if type(enabled) is not bool:
            raise ValueError('Enabled must be boolean')
        with self.db:
            self.db.execute('UPDATE preferences_settings SET enabled=? WHERE id=1', (int(enabled),))

    def correct(self, scope, category, destination, *, explicit=False):
        if explicit is not True:
            raise ValueError('Explicit user correction required')
        relative_path(scope); relative_path(destination)
        if category not in CATEGORIES:
            raise ValueError('Invalid category')
        if not self.enabled:
            return False
        with self.db:
            self.db.execute('INSERT INTO corrections VALUES(?,?,?) ON CONFLICT(scope,category) '
                            'DO UPDATE SET destination=excluded.destination', (scope, category, destination))
        return True

    def destination(self, scope, category):
        relative_path(scope)
        if category not in CATEGORIES:
            raise ValueError('Invalid category')
        if not self.enabled:
            return None
        row = self.db.execute('SELECT destination FROM corrections WHERE scope=? AND category=?',
                              (scope, category)).fetchone()
        return row[0] if row else None

    def clear(self):
        with self.db:
            self.db.execute('DELETE FROM corrections')
            # Clearing deletes logical history rather than guaranteeing media erasure
