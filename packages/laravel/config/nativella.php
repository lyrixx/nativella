<?php

return [
    /*
     * In Nativella, store the "sqlite" connection in the data directory of
     * the application (<data directory>/database.sqlite), and make it the
     * default one. Outside Nativella, the database configuration applies.
     */
    'database' => true,

    /*
     * Run "migrate --force" when the application starts, on
     * Nativella\Event\AppBooted: a desktop application has no deployment step.
     */
    'migrate_on_boot' => true,
];
