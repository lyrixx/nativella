<?php

namespace Nativella\Laravel\Facades;

use Illuminate\Support\Facades\Facade;

/**
 * Reads a barcode or a QR code with the camera of the phone.
 *
 * @method static ?\Nativella\BarcodeScanner\Barcode scan(list<\Nativella\BarcodeScanner\Format> $formats = [], string $cancelLabel = 'Cancel')
 *
 * @see \Nativella\BarcodeScanner
 */
final class BarcodeScanner extends Facade
{
    protected static function getFacadeAccessor(): string
    {
        return \Nativella\BarcodeScanner::class;
    }
}
