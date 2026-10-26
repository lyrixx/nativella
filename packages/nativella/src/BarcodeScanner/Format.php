<?php

namespace Nativella\BarcodeScanner;

/**
 * The kinds of barcodes the camera reads. The values are the names of the
 * plugin of Tauri.
 */
enum Format: string
{
    case Aztec = 'AZTEC';
    /** Android only. */
    case Codabar = 'CODABAR';
    case Code128 = 'CODE_128';
    case Code39 = 'CODE_39';
    case Code93 = 'CODE_93';
    case DataMatrix = 'DATA_MATRIX';
    case Ean13 = 'EAN_13';
    case Ean8 = 'EAN_8';
    case Itf = 'ITF';
    case Pdf417 = 'PDF_417';
    case QrCode = 'QR_CODE';
    /** Android only: iOS reads these as EAN-13, with a leading zero. */
    case UpcA = 'UPC_A';
    case UpcE = 'UPC_E';
}
