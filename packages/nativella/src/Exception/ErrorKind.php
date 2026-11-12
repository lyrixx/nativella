<?php

namespace Nativella\Exception;

/**
 * Why a native call failed: what the application can act on, next to the
 * message, which is for people.
 *
 * Keep in sync with ErrorKind in crates/runtime/src/bridge.rs.
 */
enum ErrorKind: string
{
    /** The system, or a library, failed: anything the other kinds do not say. */
    case Failed = 'failed';
    /** The payload is not what the method reads: a string that is not UTF-8, a value out of range, a path that is not absolute. */
    case InvalidPayload = 'invalidPayload';
    /** What the call names does not exist: a window, a menu item, a process, a file. */
    case NotFound = 'notFound';
    /** The user, or a device, did not answer in time. */
    case Timeout = 'timeout';
    /** The method does not exist here: on this platform, in this build, or without the permission or the configuration it needs. */
    case Unavailable = 'unavailable';
}
