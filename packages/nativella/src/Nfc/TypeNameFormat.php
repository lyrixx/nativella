<?php

namespace Nativella\Nfc;

/**
 * How the type of an NDEF record reads.
 */
enum TypeNameFormat: int
{
    case Empty = 0;
    /** A type of the NFC Forum: "T" for a text, "U" for a URI. */
    case WellKnown = 1;
    /** A MIME type, as "text/vcard". */
    case Media = 2;
    /** The type is a URI. */
    case AbsoluteUri = 3;
    /** A type of its issuer, as "example.com:notes". */
    case External = 4;
    case Unknown = 5;
    case Unchanged = 6;
}
