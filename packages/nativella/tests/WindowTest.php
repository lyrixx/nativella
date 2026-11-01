<?php

namespace Nativella\Tests;

use Nativella\Client\Call;
use Nativella\Client\FakeClient;
use Nativella\Window;
use Nativella\Window\WindowOptions;
use PHPUnit\Framework\TestCase;

final class WindowTest extends TestCase
{
    public function testOpenSendsTheWindowAndTheOptionsThatAreSet(): void
    {
        $client = new FakeClient();

        $info = new Window($client)->open('settings', new WindowOptions(url: '/settings', title: 'Settings', width: 600, resizable: false));

        $this->assertSame(['window.open'], array_column($client->calls(), 'method'));
        // A float, as the runtime reads it.
        $this->assertSame(['window' => 'settings', 'url' => '/settings', 'title' => 'Settings', 'width' => 600.0, 'resizable' => false], $client->calls()[0]->payload);
        $this->assertSame('settings', $info->window);
    }

    public function testMethodsTargetTheCurrentWindowByDefault(): void
    {
        $client = new FakeClient();
        $window = new Window($client);

        $window->setTitle('Hello');
        $window->maximize('main');

        $this->assertEquals([
            new Call('window.setTitle', ['title' => 'Hello', 'window' => null]),
            new Call('window.maximize', ['window' => 'main']),
        ], $client->calls());
    }

    public function testAllReturnsWindowInfos(): void
    {
        $client = new FakeClient();
        $client->respond('window.all', ['windows' => [[
            'window' => 'main', 'title' => 'Notes', 'url' => 'php://localhost/', 'width' => 800, 'height' => 600,
            'x' => 10, 'y' => 20, 'focused' => true, 'maximized' => false, 'minimized' => false, 'fullscreen' => false, 'visible' => true,
        ]]]);

        $windows = new Window($client)->all();

        $this->assertCount(1, $windows);
        $this->assertSame('Notes', $windows[0]->title);
        $this->assertSame(800.0, $windows[0]->width);
    }

    public function testThePageGoesToAPdf(): void
    {
        $client = new FakeClient();

        new Window($client)->printToPdf('/home/me/invoice.pdf', 'invoice');

        $this->assertEquals([new Call('window.printToPdf', ['window' => 'invoice', 'path' => '/home/me/invoice.pdf'])], $client->calls());
    }
}
