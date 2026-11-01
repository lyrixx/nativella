<?php

namespace Nativella\Tests;

use Nativella\Client\Call;
use Nativella\Client\FakeClient;
use Nativella\Event\EventFactory;
use Nativella\Event\TrayClicked;
use Nativella\Menu\MenuItem;
use Nativella\Menu\Role;
use Nativella\Tray;
use PHPUnit\Framework\TestCase;

final class TrayTest extends TestCase
{
    public function testTheIconAndItsMenu(): void
    {
        $client = new FakeClient();
        $tray = new Tray($client);

        $tray->set('Notes: 3 notes', [MenuItem::action('show', 'Show Notes'), MenuItem::role(Role::Quit)], '/app/tray.png', '3');
        $tray->remove();

        $this->assertEquals([
            new Call('tray.set', [
                'tooltip' => 'Notes: 3 notes',
                'menu' => [
                    ['type' => 'action', 'id' => 'show', 'label' => 'Show Notes', 'enabled' => true],
                    ['type' => 'role', 'role' => 'quit'],
                ],
                'icon' => '/app/tray.png',
                'title' => '3',
            ]),
            new Call('tray.remove', []),
        ], $client->calls());
    }

    public function testAFilteredMenuIsStillAList(): void
    {
        $client = new FakeClient();

        new Tray($client)->set(menu: array_filter([MenuItem::separator(), null, MenuItem::role(Role::Quit)]));

        $this->assertSame([['type' => 'separator'], ['type' => 'role', 'role' => 'quit']], $client->calls()[0]->payload['menu']);
    }

    public function testAClick(): void
    {
        $this->assertEquals(new TrayClicked('left'), EventFactory::create('tray.clicked', ['button' => 'left']));
    }
}
