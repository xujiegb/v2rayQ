include $(TOPDIR)/rules.mk

PKG_NAME:=luci-app-v2rayq
PKG_VERSION:=0.1.0
PKG_RELEASE:=1
PKG_LICENSE:=BSD-3-Clause

include $(INCLUDE_DIR)/package.mk

define Package/luci-app-v2rayq
  SECTION:=luci
  CATEGORY:=LuCI
  SUBMENU:=3. Applications
  TITLE:=v2rayQ lightweight client UI
  DEPENDS:=+luci-base
  PKGARCH:=all
endef

define Package/luci-app-v2rayq/description
  Small LuCI JavaScript interface for v2rayQ client management.
endef

define Build/Compile
endef

define Package/luci-app-v2rayq/install
	$(INSTALL_DIR) $(1)/www/luci-static/resources/view/v2rayq
	$(INSTALL_DATA) ./htdocs/luci-static/resources/view/v2rayq/client.js $(1)/www/luci-static/resources/view/v2rayq/client.js
	$(INSTALL_DIR) $(1)/www/luci-static/resources
	$(INSTALL_DATA) ./htdocs/luci-static/resources/v2rayq.css $(1)/www/luci-static/resources/v2rayq.css
	$(INSTALL_DIR) $(1)/usr/share/luci/menu.d
	$(INSTALL_DATA) ./root/usr/share/luci/menu.d/luci-app-v2rayq.json $(1)/usr/share/luci/menu.d/luci-app-v2rayq.json
endef

$(eval $(call BuildPackage,luci-app-v2rayq))
