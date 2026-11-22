'use strict';
'require view';

// Native LuCI client shell. No UI actions call a privileged backend yet.
return view.extend({
    load: function() {
        return Promise.resolve();
    },
    render: function() {
        if (!document.getElementById('v2rayq-style')) {
            var css = document.createElement('link');
            css.id = 'v2rayq-style';
            css.rel = 'stylesheet';
            css.href = L.resource('v2rayq.css');
            document.head.appendChild(css);
        }
        var sections = [
            ['Overview', '⌂', 'Local connection status'],
            ['Nodes', '◈', 'Manage subscriptions and endpoints'],
            ['Routing', '⇄', 'Direct, global and rule-based'],
            ['Settings', '⚙', 'System proxy and TUN preferences']
        ];
        var title = E('h2', {}, 'Overview');
        var subtitle = E('p', {'class':'vq-muted'}, 'Local connection status');
        var main = E('div', {'class':'vq-main'}, [
            title, subtitle,
            E('div', {'class':'vq-card'}, [
                E('span', {'class':'vq-chip'}, 'Core disconnected'),
                E('h3', {}, 'Waiting for client adapter'),
                E('p', {'class':'vq-muted'}, 'This UI is a read-only preview. No network settings are changed.')
            ])
        ]);
        var buttons = sections.map(function(section, index) {
            var button = E('button', {'type':'button',
                'class':'vq-nav' + (index === 0 ? ' selected' : '')
            }, [E('span', {}, section[1]), section[0]]);
            button.addEventListener('click', function() {
                buttons.forEach(function(item) { item.classList.remove('selected'); });
                button.classList.add('selected');
                title.textContent = section[0];
                subtitle.textContent = section[2];
            });
            return button;
        });
        return E('div', {'class':'vq-shell'}, [
            E('aside', {'class':'vq-side'}, [
                E('div', {'class':'vq-logo'}, '◈  v2rayQ'),
                E('div', {'class':'vq-subheading'}, 'CLIENT'),
                E('nav', {'class':'vq-navigation'}, buttons),
                E('div', {'class':'vq-footnote'}, 'Read-only preview')
            ]),
            main
        ]);
    },
    handleSave: null,
    handleSaveApply: null,
    handleReset: null
});
