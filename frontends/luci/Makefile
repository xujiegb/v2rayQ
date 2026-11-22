include $(TOPDIR)/rules.mk

PKG_NAME:=luci-app-v2rayq
PKG_VERSION:=0.1.0
PKG_RELEASE:=3
PKG_LICENSE:=BSD-3-Clause

include $(INCLUDE_DIR)/package.mk

define Package/luci-app-v2rayq
  SECTION:=luci
  CATEGORY:=LuCI
  SUBMENU:=3. Applications
  TITLE:=v2rayQ lightweight client UI
  DEPENDS:=+luci-base +rpcd
  PKGARCH:=all
endef

define Package/luci-app-v2rayq/description
  Small LuCI frontend. Install v2rayq-cli in /usr/bin for the target architecture.
endef

define Build/Compile
	python3 $(CURDIR)/../../tools/prepare_locales.py client $(PKG_BUILD_DIR)/locales
endef

define Package/luci-app-v2rayq/install
	$(INSTALL_DIR) $(1)/www/luci-static/resources/view/v2rayq
	$(INSTALL_DATA) ./htdocs/luci-static/resources/view/v2rayq/client.js $(1)/www/luci-static/resources/view/v2rayq/client.js
	$(INSTALL_DIR) $(1)/www/luci-static/resources/v2rayq
	$(INSTALL_DATA) ./htdocs/luci-static/resources/v2rayq/* $(1)/www/luci-static/resources/v2rayq/
	$(INSTALL_DIR) $(1)/www/luci-static/resources/v2rayq/locales
	$(INSTALL_DATA) $(PKG_BUILD_DIR)/locales/* $(1)/www/luci-static/resources/v2rayq/locales/
	$(INSTALL_DIR) $(1)/usr/share/luci/menu.d $(1)/usr/share/rpcd/acl.d $(1)/usr/libexec/rpcd
	$(INSTALL_DATA) ./root/usr/share/luci/menu.d/luci-app-v2rayq.json $(1)/usr/share/luci/menu.d/
	$(INSTALL_DATA) ./root/usr/share/rpcd/acl.d/luci-app-v2rayq.json $(1)/usr/share/rpcd/acl.d/
	$(INSTALL_BIN) ./root/usr/libexec/rpcd/v2rayq $(1)/usr/libexec/rpcd/
endef

$(eval $(call BuildPackage,luci-app-v2rayq))
