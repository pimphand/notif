/**
 * NotifMoo — JavaScript client for Notif WebSocket (Pusher-like).
 * Include: <script src="notifmoo.js?apikey=YOUR_API_KEY"></script>
 * Optional: &host=wss://your-server.com (default: same origin)
 */
(function (global) {
  'use strict';

  var script = document.currentScript || (function () {
    var scripts = document.getElementsByTagName('script');
    return scripts[scripts.length - 1];
  })();

  var src = script && script.src ? script.src : '';
  var params = {};
  if (src) {
    var q = src.indexOf('?');
    if (q !== -1) {
      var search = new URLSearchParams(src.slice(q));
      search.forEach(function (v, k) {
        params[k.toLowerCase()] = v;
      });
    }
  }

  var apikey = params.apikey || params.api_key || '';
  var hostParam = params.host || '';
  var wsUrl;

  if (hostParam) {
    var base = hostParam.replace(/\/+$/, '');
    wsUrl = base + '/ws' + (apikey ? '?api_key=' + encodeURIComponent(apikey) : '');
  } else {
    wsUrl = 'wss://notification.officialconnect.id/ws' + (apikey ? '?api_key=' + encodeURIComponent(apikey) : '');
  }

  var ws = null;
  var socketId = null;
  var readyState = 'closed';
  var subscriptions = {};
  var eventBindings = {};
  var connectionCallbacks = [];
  var subscriptionSucceededCallbacks = {};
  var errorCallbacks = [];

  function setReadyState(s) {
    readyState = s;
    if (s === 'connected') {
      connectionCallbacks.forEach(function (cb) {
        try { cb(); } catch (e) {}
      });
    }
  }

  function send(obj) {
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(obj));
    }
  }

  function subscribe(channel, opts) {
    opts = opts || {};
    var auth = opts.auth;
    var channelData = opts.channelData;
    var onMessage = opts.onMessage;

    if (!subscriptions[channel]) {
      subscriptions[channel] = { onMessage: [] };
    }
    if (onMessage) {
      subscriptions[channel].onMessage.push(onMessage);
    }

    var payload = { event: 'subscribe', data: { channel: channel } };
    if (auth) payload.data.auth = auth;
    if (channelData) payload.data.channel_data = channelData;
    send(payload);
  }

  function unsubscribe(channel) {
    send({ event: 'unsubscribe', data: { channel: channel } });
    delete subscriptions[channel];
    delete subscriptionSucceededCallbacks[channel];
  }

  function bind(eventName, callback) {
    if (!eventBindings[eventName]) eventBindings[eventName] = [];
    eventBindings[eventName].push(callback);
  }

  function unbind(eventName, callback) {
    if (!eventBindings[eventName]) return;
    if (!callback) {
      eventBindings[eventName] = [];
      return;
    }
    eventBindings[eventName] = eventBindings[eventName].filter(function (cb) { return cb !== callback; });
  }

  function onError(callback) {
    errorCallbacks.push(callback);
  }

  function trigger(eventName, payload) {
    var list = eventBindings[eventName];
    if (list) {
      list.forEach(function (cb) {
        try { cb(payload); } catch (e) {}
      });
    }
    var ch = payload.channel;
    if (ch && subscriptions[ch] && subscriptions[ch].onMessage) {
      subscriptions[ch].onMessage.forEach(function (cb) {
        try { cb(payload); } catch (e) {}
      });
    }
  }

  function connect() {
    if (ws && (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING)) {
      return;
    }
    setReadyState('connecting');
    ws = new WebSocket(wsUrl);

    ws.onopen = function () {
      setReadyState('connected');
    };

    ws.onclose = function () {
      setReadyState('closed');
      ws = null;
      socketId = null;
    };

    ws.onerror = function () {
      setReadyState('error');
      errorCallbacks.forEach(function (cb) {
        try { cb({ message: 'WebSocket error' }); } catch (e) {}
      });
    };

    ws.onmessage = function (ev) {
      try {
        var msg = JSON.parse(ev.data);
        var event = msg.event;
        var data = msg.data;

        if (event === 'connection_established' && data && data.socket_id) {
          socketId = data.socket_id;
          trigger('connection_established', { socket_id: socketId });
          return;
        }

        if (event === 'pusher_internal:subscription_succeeded') {
          var channel = msg.channel;
          var cb = subscriptionSucceededCallbacks[channel];
          if (cb) {
            try { cb(data || {}); } catch (e) {}
          }
          trigger('subscription_succeeded', { channel: channel, data: data || {} });
          return;
        }

        if (event === 'pusher:error') {
          var errPayload = { message: (data && data.message) || 'Unknown error', code: (data && data.code) };
          errorCallbacks.forEach(function (cb) {
            try { cb(errPayload); } catch (e) {}
          });
          trigger('pusher:error', errPayload);
          return;
        }

        var payload = {
          event: event,
          channel: msg.channel || '',
          data: data
        };
        trigger(event, payload);
        trigger('*', payload);
      } catch (e) {
        trigger('*', { raw: ev.data });
      }
    };
  }

  var NotifMoo = {
    connect: connect,
    subscribe: subscribe,
    unsubscribe: unsubscribe,
    bind: bind,
    unbind: unbind,
    onError: onError,
    onConnect: function (callback) {
      if (readyState === 'connected') {
        try { callback(); } catch (e) {}
      } else {
        connectionCallbacks.push(callback);
      }
    },
    onSubscriptionSucceeded: function (channel, callback) {
      subscriptionSucceededCallbacks[channel] = callback;
    },
    get socketId() { return socketId; },
    get readyState() { return readyState; },
    get apikey() { return apikey; }
  };

  connect();

  global.NotifMoo = NotifMoo;
})(typeof window !== 'undefined' ? window : this);
