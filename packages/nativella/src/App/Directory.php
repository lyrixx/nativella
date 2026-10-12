<?php

namespace Nativella\App;

/**
 * The directories App::path() knows.
 */
enum Directory: string
{
    case AppCache = 'appCache';
    case AppConfig = 'appConfig';
    case AppData = 'appData';
    case AppLog = 'appLog';
    case Desktop = 'desktop';
    case Documents = 'documents';
    case Downloads = 'downloads';
    case Home = 'home';
    case Music = 'music';
    case Pictures = 'pictures';
    case Temp = 'temp';
    case Videos = 'videos';
}
