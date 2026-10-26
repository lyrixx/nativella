<?php

namespace Nativella\Tests;

use Nativella\BarcodeScanner;
use Nativella\BarcodeScanner\Format;
use Nativella\Client\FakeClient;
use PHPUnit\Framework\TestCase;

final class BarcodeScannerTest extends TestCase
{
    public function testTheCodeTheCameraReads(): void
    {
        $client = new FakeClient();
        $client->respond('barcodeScanner.scan', ['barcode' => ['content' => '3017620422003', 'format' => 'EAN_13']]);

        $barcode = new BarcodeScanner($client)->scan([Format::Ean13, Format::QrCode], 'Annuler');

        $this->assertNotNull($barcode);
        $this->assertSame('3017620422003', $barcode->content);
        $this->assertSame(Format::Ean13, $barcode->format);
        $this->assertSame(
            ['formats' => ['EAN_13', 'QR_CODE'], 'cancelLabel' => 'Annuler'],
            $client->calls('barcodeScanner.scan')[0]['payload'],
        );
    }

    public function testEveryFormatByDefault(): void
    {
        $client = new FakeClient();

        new BarcodeScanner($client)->scan();

        $this->assertSame(['formats' => [], 'cancelLabel' => 'Cancel'], $client->calls('barcodeScanner.scan')[0]['payload']);
    }

    public function testAFormatThePluginKnowsAndWeDoNot(): void
    {
        $client = new FakeClient();
        $client->respond('barcodeScanner.scan', ['barcode' => ['content' => '0123', 'format' => 'GS1_DATA_BAR']]);

        $barcode = new BarcodeScanner($client)->scan();

        $this->assertNotNull($barcode);
        $this->assertNull($barcode->format);
    }

    public function testTheUserMayRefuseOrCancel(): void
    {
        $this->assertNull(new BarcodeScanner(new FakeClient())->scan());
    }
}
