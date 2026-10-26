<?php

namespace Nativella;

use Nativella\BarcodeScanner\Barcode;
use Nativella\BarcodeScanner\Format;

/**
 * Reads a barcode or a QR code with the camera of the phone.
 *
 * The application asks for it with "barcodeScanner" in the permissions of
 * extra.nativella, in its composer.json; the user is asked on the first call.
 */
final readonly class BarcodeScanner
{
    public function __construct(
        private Client $client,
    ) {
    }

    /**
     * Shows the camera in the window, behind the page, and blocks until it
     * reads a code, or the user cancels. Null when the user does not allow
     * the camera, or cancels.
     *
     * On Android, QR codes are read whatever the formats.
     *
     * @param list<Format> $formats     the kinds of codes to read, every kind when empty
     * @param string       $cancelLabel the label of the button that gives up, in the language of the application
     *
     * @throws Exception\NativeCallException when the camera fails, or reads no code within 2 minutes
     */
    public function scan(array $formats = [], string $cancelLabel = 'Cancel'): ?Barcode
    {
        $barcode = $this->client->call('barcodeScanner.scan', [
            'formats' => array_map(static fn (Format $format): string => $format->value, $formats),
            'cancelLabel' => $cancelLabel,
        ])['barcode'];

        return null === $barcode ? null : Barcode::fromArray($barcode);
    }
}
