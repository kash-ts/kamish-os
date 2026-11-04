################################################################################
#
# kamish-healthcheck
#
################################################################################

KAMISH_HEALTHCHECK_VERSION = 1.0.0
KAMISH_HEALTHCHECK_SITE = $(BR2_EXTERNAL_KAMISH_PATH)/board/kamish/orangepi-zero3/overlay
KAMISH_HEALTHCHECK_SITE_METHOD = local

define KAMISH_HEALTHCHECK_INSTALL_TARGET_CMDS
	$(INSTALL) -D -m 0755 $(@D)/usr/sbin/kamish-healthcheck \
		$(TARGET_DIR)/usr/sbin/kamish-healthcheck
	$(INSTALL) -D -m 0644 $(@D)/etc/systemd/system/kamish-healthcheck.service \
		$(TARGET_DIR)/etc/systemd/system/kamish-healthcheck.service
	mkdir -p $(TARGET_DIR)/etc/systemd/system/multi-user.target.wants
	ln -sf /etc/systemd/system/kamish-healthcheck.service \
		$(TARGET_DIR)/etc/systemd/system/multi-user.target.wants/kamish-healthcheck.service
endef

$(eval $(generic-package))
