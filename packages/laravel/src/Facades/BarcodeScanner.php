<?php

namespace Nativella\Laravel\Facades;

use Illuminate\Support\Facades\Facade;

/**
 * @method static ?\Nativella\BarcodeScanner\Barcode scan(array<\Nativella\BarcodeScanner\Format> $formats = [], string $cancelLabel = 'Cancel')
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
