'use strict';
'require view';
'require rpc';

const profile = rpc.declare({ object: 'v2rayq', method: 'profile', params: ['command'], reject: true });
const service = rpc.declare({ object: 'v2rayq', method: 'service', params: ['command'], reject: true });
const serviceAccess = rpc.declare({ object: 'session', method: 'access', params: ['scope', 'object', 'function'], expect: { access: false } });

return view.extend({
    load: function() {
        return Promise.all([
            import(L.resource('v2rayq/client-ui.js')),
            serviceAccess('ubus', 'v2rayq', 'service')
        ]);
    },

    render: function(loaded) {
        const [ui, canControl] = loaded;
        const root = E('div', { 'class': 'vq-client' });
        ui.mountClient(root, async function(command) {
            const call = command.action === 'set_network' ? service : profile;
            const reply = await call(JSON.stringify(command));
            if (reply.error) throw new Error(reply.error);
            return reply.result;
        }, { delegatedControl: canControl === true });
        return E('div', {}, [E('link', { rel: 'stylesheet', href: L.resource('v2rayq/client.css') }), root]);
    },

    handleSaveApply: null,
    handleSave: null,
    handleReset: null
});
